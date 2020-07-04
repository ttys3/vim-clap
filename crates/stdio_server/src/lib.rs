mod env;
mod filer;
mod session;
mod types;

use crossbeam_channel::{Receiver, Sender};
use log::{debug, error};
use serde::Serialize;
use serde_json::json;
use session::{Manager, SessionEvent};
use std::io::prelude::*;
use std::thread;
use types::Message;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn write_response<T: Serialize>(msg: T) {
    if let Ok(s) = serde_json::to_string(&msg) {
        println!("Content-length: {}\n\n{}", s.len(), s);
    }
}

fn loop_read_rpc_message(reader: impl BufRead, sink: &Sender<String>) {
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

// Runs in the main thread.
fn loop_handle_rpc_message(rx: &Receiver<String>) {
    let mut session_manager = Manager::default();
    for msg in rx.iter() {
        if let Ok(msg) = serde_json::from_str::<Message>(&msg.trim()) {
            debug!("Recv: {:?}", msg);
            match &msg.method[..] {
                "filer" => filer::handle_message(msg),
                "filer/on_init" => {
                    session_manager.new_session(msg.session_id, msg, filer::FilerSession)
                }
                "initialize_global_env" => env::initialize_global(msg),
                "on_init" => session_manager.new_opaque_session(msg.session_id, msg),
                "on_typed" => session_manager.send(msg.session_id, SessionEvent::OnTyped(msg)),
                "on_move" | "filer/on_move" => {
                    session_manager.send(msg.session_id, SessionEvent::OnMove(msg))
                }
                "exit" => session_manager.terminate(msg.session_id),
                _ => write_response(
                    json!({ "error": format!("unknown method: {}", &msg.method[..]), "id": msg.id }),
                ),
            }
        } else {
            error!("Invalid message: {:?}", msg);
        }
    }
}

/// v0.1.19 -> 19
#[inline]
fn parse_running_version() -> &'static str {
    VERSION
        .split('.')
        .last()
        .expect("wrong stdio_server cargo version")
}

fn should_check_new_release() -> anyhow::Result<()> {
    use std::time::SystemTime;
    let mut checked_cache = utility::clap_cache_dir();
    checked_cache.push("maple_release_last_checked");
    // TODO:
    // 1. get last checked time.
    // 2. if checked recently skipped,
    // 3. try upgrading
    if checked_cache.exists() {
        if let Ok(mut lines_iter) = utility::read_first_lines(&checked_cache, 1) {
            if let Some(line) = lines_iter.next() {
                let now = format!(
                    "{:?}",
                    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?
                );
                std::fs::File::create(&checked_cache)?.write_all(now.as_bytes())?;
            }
        }
    }
    Ok(())
}

fn uprade_release_binary() {
    if upgrade::running_from_bin_dir().is_ok() {
        // v0.19
        if let Ok(tag_name) = upgrade::UpgradeDaemon::get_latest_release() {
            // 19
            if let Some(remote_version) = tag_name.split('.').last() {
                let running_version = parse_running_version();
                if remote_version == running_version {
                    debug!("already running the latest GitHub release!");
                } else {
                    debug!(
                        "not running the latest GitHub release, local: {}, latest: {}",
                        running_version, remote_version
                    );
                    upgrade::download_latest_github_release(&tag_name);
                }
            }
        }
    } else {
        debug!("using the locally compiled binary, good for you!");
    }
}

pub fn run_forever<R>(reader: R)
where
    R: BufRead + Send + 'static,
{
    let (tx, rx) = crossbeam_channel::unbounded();

    thread::Builder::new()
        .name("reader".into())
        .spawn(move || {
            loop_read_rpc_message(reader, &tx);
        })
        .expect("Failed to spawn rpc reader thread");

    thread::Builder::new()
        .name("upgrade-release-binary".into())
        .spawn(move || uprade_release_binary())
        .expect("Failed to spawn upgrade-release-binary thread");

    loop_handle_rpc_message(&rx);
}
