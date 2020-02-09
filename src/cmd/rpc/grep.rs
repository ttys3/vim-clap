use std::path::PathBuf;
use tokio::process::Command;
use tokio::runtime::Runtime;

use super::Message;

pub(super) fn handle_message(msg: Message) {
    if let Some(query) = msg.params.get("query").and_then(|x| x.as_str()) {
        let mut runtime = Runtime::new().unwrap();
        let mut cmd = Command::new("rg");
        cmd.args(&["-H", "--no-heading", "--vimgrep", "--smart-case"]);
        cmd.arg(query);
        let dir: Option<PathBuf> = msg
            .params
            .get("dir")
            .and_then(|x| x.as_str())
            .map(Into::into);
        super::async_cmd::set_current_dir(&mut cmd, dir);
        let _ = runtime.block_on(super::async_cmd::run(cmd, msg.id));
    }
}
