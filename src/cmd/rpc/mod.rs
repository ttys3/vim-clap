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
    let mut initial_size = 0usize;
    let mut data_received = false;

    for msg in rx.iter() {
        if !data_received {
            if let Ok(data) = stdout_recv.try_recv() {
                initial_size = bytecount::count(&data, b'\n');
                raw_data = data;
                data_received = true;
            }
        }
        if data_received {
            if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
                if let Some(query) = msg.params.get("query").and_then(|x| x.as_str()) {
                    let lines = crate::cmd::filter::filter(
                        String::from_utf8_lossy(&raw_data).split('\n'),
                        query,
                        None,
                    );
                    if let Ok(lines) = lines {
                        let (total, lines, indices) = crate::cmd::filter::truncate(lines);
                        write_response(
                            json!({ "result": { "lines": lines, "total": total, "initial_size": initial_size, "indices": indices }, "id": msg.id }),
                        );
                    }
                }
            }
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

fn spawn_for_filtering(stdout_send: crossbeam_channel::Sender<Vec<u8>>) {
    // Spawn the command async
    // Collect the whole stdout
    thread::Builder::new()
        .name("run-for-complete-stdout".into())
        .spawn(move || {
            let mut cmd = std::process::Command::new("bash");
            cmd.args(&["-c", "fd --type f"]);

            crate::light_command::set_current_dir(&mut cmd, Some("/Users/xuliucheng".into()));

            let cmd_output = cmd.output().expect("Gather stdout");

            if !cmd_output.status.success() && !cmd_output.stderr.is_empty() {
                let error = format!("{}", String::from_utf8_lossy(&cmd_output.stderr));
            }

            stdout_send
                .send(cmd_output.stdout)
                .expect("Failed to send the whole stdout");

            drop(stdout_send);
        })
        .expect("Failed to spawn rpc reader thread");
}

pub fn run_forever<R>(reader: R)
where
    R: BufRead + Send + 'static,
{
    let (tx, rx) = crossbeam_channel::unbounded();
    let (stdout_send, stdout_recv) = crossbeam_channel::bounded(1);

    spawn_for_filtering(stdout_send);

    thread::Builder::new()
        .name("reader".into())
        .spawn(move || {
            loop_read(reader, &tx);
        })
        .expect("Failed to spawn rpc reader thread");

    loop_handle_message(&rx, &stdout_recv);
}
