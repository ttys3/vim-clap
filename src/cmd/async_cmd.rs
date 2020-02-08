use async_std::task;
use std::error::Error;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{self, Duration};

use crate::cmd::rpc::write_response;

async fn refresh(count: Arc<AtomicUsize>, running: Arc<bool>) {
    let mut interval = time::interval(Duration::from_millis(50));
    loop {
        interval.tick().await;
        let result = serde_json::json!({ "total": format!("{:?}", count) });
        write_response(serde_json::json!({ "result": result, "id": 1 }));
    }
}

async fn read_output(
    stdout: tokio::process::ChildStdout,
    cnt: Arc<AtomicUsize>,
    running: Arc<bool>,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stdout).lines();

    let mut top_n = Vec::new();
    let mut sent_top_n = false;

    while let Some(line) = reader.next_line().await? {
        let new = cnt.fetch_add(1, Ordering::SeqCst);
        if !sent_top_n {
            if new < 500 {
                top_n.push(line);
            } else {
                sent_top_n = true;
                let result = serde_json::json!({ "lines": top_n });
                write_response(serde_json::json!({ "result": result, "id": 1 }));
                // println!("Lines: {:?}", top_n);
            }
        }
    }

    let result = serde_json::json!({ "total": format!("{:?}", cnt) });
    write_response(serde_json::json!({ "result": result, "id": 1 }));
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

async fn async_run(cmd: Command) {
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
    let running = Arc::new(true);

    tokio::spawn(refresh(cnt.clone(), running.clone()));

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    tokio::spawn(async {
        let status = child.await.expect("child process encountered an error");
        // println!("child status was: {}", status);
    });

    if task::block_on(read_output(stdout, cnt, running)).is_err() {
        println!("ERROR");
    }
}

pub async fn run(cmd: Command) -> Result<(), Box<dyn Error>> {
    task::block_on(async_run(cmd));
    Ok(())
}
