mod filer;
mod files;
mod grep;

use std::io::prelude::*;
use std::thread;

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const REQUEST_FILER: &str = "filer";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub method: String,
    pub params: serde_json::Map<String, Value>,
    pub id: u64,
}

pub fn write_response<T: Serialize>(msg: T) {
    if let Ok(s) = serde_json::to_string(&msg) {
        println!("Content-length: {}\n\n\n{}", s.len(), s);
    }
}

fn loop_read(reader: impl BufRead, sink: &Sender<String>) {
    let mut reader = reader;
    loop {
        let mut message = String::new();
        match reader.read_line(&mut message) {
            Ok(number) => {
                if number > 0 {
                    if let Err(e) = sink.send(message) {
                        println!("Failed to send message, error: {}", e);
                    }
                } else {
                    println!("EOF reached");
                }
            }
            Err(error) => println!("Failed to read_line, error: {}", error),
        }
    }
}

fn loop_handle_message(
    rx: &crossbeam_channel::Receiver<String>,
    stdout_recv: &crossbeam_channel::Receiver<Vec<u8>>,
) {
    let mut raw_data = Vec::new();
    for msg in rx.iter() {
        if raw_data.is_empty() {
            if let Ok(data) = stdout_recv.try_recv() {
                if !data.is_empty() {
                    raw_data = data;
                }
            }
        }
        if !raw_data.is_empty() {
            // if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
            // if let Some(query) = msg.params.get("query").and_then(|x| x.as_str()) {
            println!("filering.....");
            let query = msg.trim();
            let stdout_str = String::from_utf8_lossy(&raw_data);
            let stdout = stdout_str.split('\n').collect::<Vec<_>>();
            let lines = crate::cmd::filter::filter(&stdout, query, None, None);
            println!("lines: {:?}", lines);
        // }
        // }
        // do filtering
        } else {
            thread::spawn(move || {
                // Ignore the invalid message.
                if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
                    match &msg.method[..] {
                        REQUEST_FILER => filer::handle_message(msg),
                        "grep" => grep::handle_message(msg),
                        "files" => files::handle_message(msg),
                        _ => write_response(json!({ "error": "unknown method", "id": msg.id })),
                    }
                }
            });
        }
    }
}

pub fn run_forever<R>(reader: R)
where
    R: BufRead + Send + 'static,
{
    let (tx, rx) = crossbeam_channel::unbounded();
    let (stdout_send, stdout_recv) = crossbeam_channel::bounded(1);

    // Spawn the command async
    // Collect the whole stdout
    thread::Builder::new()
        .name("run-for-complete-stdout".into())
        .spawn(move || {
            let mut cmd = std::process::Command::new("bash");
            cmd.args(&["-c", "fd --type f"]);
            let cmd_output = cmd.output().expect("Gather stdout");

            if !cmd_output.status.success() && !cmd_output.stderr.is_empty() {
                let error = format!("{}", String::from_utf8_lossy(&cmd_output.stderr));
            }

            stdout_send.send(cmd_output.stdout);
        })
        .expect("Failed to spawn rpc reader thread");

    thread::Builder::new()
        .name("reader".into())
        .spawn(move || {
            loop_read(reader, &tx);
        })
        .expect("Failed to spawn rpc reader thread");

    loop_handle_message(&rx, &stdout_recv);
}
