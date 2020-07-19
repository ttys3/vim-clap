use crate::session::{
    HandleMessage, NewSession, OnMove, OnMoveHandler, RpcMessage, Session, SessionContext,
    SessionEvent,
};
use crate::utils::build_abs_path;
use crate::{write_response, Message};
use anyhow::Result;
use crossbeam_channel::Sender;
use icon::prepend_filer_icon;
use log::debug;
// use maple_cli::cmd::tags::Tags;
use serde_json::json;
use std::path::{self, Path, PathBuf};
use std::{fs, io};

pub struct ProjTagsSession;

#[derive(Clone)]
pub struct ProjTagsMessageHandler;

impl HandleMessage for ProjTagsMessageHandler {
    fn handle(&self, msg: RpcMessage, context: &SessionContext) {
        match msg {
            RpcMessage::OnMove(msg) => {
                let provider_id = context.provider_id.clone();
                let curline = msg.get_curline(&provider_id).unwrap();
                let path = build_abs_path(&msg.get_cwd(), curline);
                let on_move_handler = OnMoveHandler {
                    msg_id: msg.id,
                    size: provider_id.get_preview_size(),
                    provider_id,
                    context,
                    inner: OnMove::Filer(path),
                };
                on_move_handler.handle().unwrap();
            }
            // TODO: handle on_typed
            RpcMessage::OnTyped(msg) => handle_message_on_typed(msg),
        }
    }
}

impl NewSession for ProjTagsSession {
    fn spawn(&self, msg: Message) -> Result<Sender<SessionEvent>> {
        let (session_sender, session_receiver) = crossbeam_channel::unbounded();

        let mut session = Session {
            session_id: msg.session_id,
            context: msg.clone().into(),
            message_handler: ProjTagsMessageHandler,
            event_recv: session_receiver,
        };

        // handle on_init
        handle_message_on_init(msg, &mut session.context);

        session.start_event_loop()?;

        Ok(session_sender)
    }
}

pub fn handle_message_on_init(msg: Message, context: &mut SessionContext) {
    debug!("handle message on init: {:?}", msg);
    // let proj_tags = Tags::default();
    // let executor = Executor::ProjTags(String::from(context.cwd));
    // if let Ok(lines) = proj_tags.run_on_init_at(context.cwd.into()) {
    let cwd = context.cwd.clone();
    let dir: PathBuf = cwd.into();
    if let Ok(lines) = executor::execute_at(executor::Executor::ProjTags, &dir) {
        debug!(
            "handle message on init, setting source list, len: {:?}",
            lines.len()
        );
        context.set_source_list(lines.clone());

        let total = lines.len();
        let result = serde_json::json!({
        "lines": lines.into_iter().take(50).collect::<Vec<_>>(),
        "total": total,
        });

        let res = json!({ "id": msg.id, "provider_id": "proj_tags", "result": result });

        debug!("----------- sending. res");
        crate::write_response(res);
    }
}

pub fn handle_message_on_typed(msg: Message) {
    debug!("handle message on typed: {:?}", msg);
}
