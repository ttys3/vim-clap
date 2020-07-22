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
            RpcMessage::OnTyped(msg) => handle_message_on_typed(msg, context),
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

pub fn handle_message_on_typed(msg: Message, context: &SessionContext) {
    use filter::Source;
    use matcher::Algo;
    use matcher::LineSplitter;

    debug!("handle message on typed: {:?}", msg);
    let query = msg.get_query();

    let source_list = context.source_list.lock().unwrap();

    /*
    if let Some(lines) = source_list.as_ref() {
        let source: Source<_> = lines.clone().into();
        if let Ok(filtered) = source.full_filter(Algo::Fzy, &query, LineSplitter::TagNameOnly) {
            let filter_response =
                printer::get_sync_filter_response(filtered, 50, context.winwidth, None);

            let result = serde_json::json!({
              "lines": filter_response.lines,
              "indices": filter_response.indices,
              "truncated_map": filter_response.truncated_map
            });

            let res = json!({ "id": msg.id, "provider_id": "proj_tags", "result": result });

            debug!("----------- sending. on_typed res");
            crate::write_response(res);
        }
    } else {

      */
    if let Ok(tags_stream) = executor::default_formatted_tags_stream(&context.cwd.clone().into()) {
        debug!("----------- on_typed dyn_run");
        filter::dyn_run(
            &query,
            Source::List(tags_stream),
            Some(Algo::Fzy),
            Some(50),
            context.winwidth.map(|x| x as usize),
            None,
            LineSplitter::TagNameOnly,
        );
    }
    // }
}
