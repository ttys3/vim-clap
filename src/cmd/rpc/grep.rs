use std::path::PathBuf;
use tokio::process::Command;
use tokio::runtime::Runtime;

use super::Message;

pub(super) fn handle_message(msg: Message) {
    if let Some(query) = msg.params.get("query").and_then(|x| x.as_str()) {
        let mut runtime = Runtime::new().unwrap();
        let mut cmd = Command::new("rg");
        // Do not use --vimgrep here.
        cmd.args(&[
            "--column",
            "--line-number",
            "--no-heading",
            "--color=never",
            "--smart-case",
        ]);
        cmd.arg(query);
        let dir: Option<PathBuf> = msg
            .params
            .get("dir")
            .and_then(|x| x.as_str())
            .map(Into::into);
        crate::live_command::set_current_dir(&mut cmd, dir);
        let _ = runtime.block_on(crate::live_command::run(cmd, msg.id));
    }
}
