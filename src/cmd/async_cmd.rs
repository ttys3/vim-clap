use async_std::task;
use std::error::Error;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{self, Duration};

use crate::cmd::rpc::write_response;

async fn refresh(count: Arc<AtomicUsize>, stop: Arc<AtomicBool>, req_id: u64) {
    let mut interval = time::interval(Duration::from_millis(30));
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        interval.tick().await;
        let result = serde_json::json!({ "total": format!("{:?}", count) });
        write_response(serde_json::json!({ "result": result, "id": req_id }));
    }
}

async fn read_output(
    stdout: tokio::process::ChildStdout,
    cnt: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    req_id: u64,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stdout).lines();

    let mut top_n = Vec::new();
    let mut top_n_sent = false;

    loop {
        match reader.next_line().await {
            Ok(o) => {
                if let Some(line) = o {
                    let prev = cnt.fetch_add(1, Ordering::SeqCst);
                    if !top_n_sent {
                        if prev + 1 < 500 {
                            top_n.push(line);
                        } else {
                            top_n_sent = true;
                            let result = serde_json::json!({ "lines": top_n });
                            write_response(serde_json::json!({ "result": result, "id": req_id }));
                        }
                    }
                } else {
                    break;
                }
            }
            Err(err) => {
                let line = format!("{:?}", err);
                let prev = cnt.fetch_add(1, Ordering::SeqCst);
                if !top_n_sent {
                    if prev + 1 < 500 {
                        top_n.push(line);
                    } else {
                        top_n_sent = true;
                        let result = serde_json::json!({ "lines": top_n });
                        write_response(serde_json::json!({ "result": result, "id": req_id }));
                    }
                }
                // Custom { kind: InvalidData, error: "stream did not contain valid UTF-8" }
                // println!("error in read_output: {:?}", err);
            }
        }
    }

    stop.store(true, Ordering::SeqCst);
    assert_eq!(stop.load(Ordering::SeqCst), true);

    let result = if top_n_sent {
        serde_json::json!({ "total": format!("{:?}", cnt) })
    } else {
        serde_json::json!({ "total": format!("{:?}", cnt), "lines": top_n })
    };

    write_response(serde_json::json!({ "result": result, "id": req_id }));

    Ok(())
}

pub fn set_current_dir(cmd: &mut Command, cmd_dir: Option<PathBuf>) {
    if let Some(cmd_dir) = cmd_dir {
        // If cmd_dir is not a directory, use its parent as current dir.
        if cmd_dir.is_dir() {
            cmd.current_dir(cmd_dir);
        } else {
            let mut cmd_dir = cmd_dir;
            cmd_dir.pop();
            cmd.current_dir(cmd_dir);
        }
    }
}

async fn async_run(cmd: Command, req_id: u64) {
    let mut cmd = cmd;
    // Specify that we want the command's standard output piped back to us.
    // By default, standard input/output/error will be inherited from the
    // current process (for example, this means that standard input will
    // come from the keyboard and standard output/error will go directly to
    // the terminal if this process is invoked from the command line).
    cmd.stdout(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn command");

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let cnt = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    tokio::spawn(async {
        if let Err(err) = child.await {
            println!("error:{:?}", err);
            // write_response(serde_json::json!({ "error": format!("{}", err) }));
        }
    });

    tokio::spawn(read_output(stdout, cnt.clone(), Arc::clone(&stop), req_id));

    task::block_on(refresh(cnt, stop, req_id));

    std::process::exit(0);
}

pub async fn run(cmd: Command, req_id: u64) -> Result<(), Box<dyn Error>> {
    task::block_on(async_run(cmd, req_id));
    Ok(())
}
