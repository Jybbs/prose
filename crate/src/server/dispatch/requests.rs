//! LSP request routing: the `textDocument/formatting` handler.

use lsp_server::{Connection, ErrorCode, ExtractError, Message, Request, Response};
use lsp_types::{
    DocumentFormattingParams,
    request::{Formatting, Request as RequestTrait},
};
use ruff_source_file::PositionEncoding;

use super::send;
use crate::server::{
    analysis::{self, Formatted},
    config_cache::ConfigCache,
    documents::DocumentStore,
    notices::Notices,
};
/// Routes one request, answering formatting and rejecting any other
/// method so the client never blocks waiting for a response. The settle
/// check itself runs only after the edits response is on the wire, and
/// only for a document not yet holding its once-per-session notice, so
/// neither detection nor narrowing sits on the formatting latency.
pub(super) fn handle_request(
    connection: &Connection,
    documents: &DocumentStore,
    configs: &mut ConfigCache,
    notices: &mut Notices,
    request: Request,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    let id = request.id.clone();
    match request.extract::<DocumentFormattingParams>(Formatting::METHOD) {
        Ok((id, params)) => {
            // The client's `FormattingOptions` (tab size, spaces) go unused
            // because prose formats to its own `[tool.prose]` config, not
            // editor settings.
            let uri = &params.text_document.uri;
            let doc = documents.get(uri);
            let config = doc.map(|doc| configs.resolve(uri, &doc.text));
            let Formatted { edits, settled } = doc
                .zip(config.as_ref())
                .map_or_else(Formatted::default, |(doc, config)| {
                    analysis::format_buffer(&doc.text, encoding, config)
                });
            send(connection, Message::Response(Response::new_ok(id, edits)))?;
            let Some(((doc, config), settled)) = doc.zip(config).zip(settled) else {
                return Ok(());
            };
            if notices.reported(uri) || !config.report_unstable_output {
                return Ok(());
            }
            settled
                .detect(&config, &doc.text)
                .map_or(Ok(()), |rewrite| {
                    notices.offer(connection, uri, &doc.text, &rewrite)
                })
        }
        Err(ExtractError::MethodMismatch(request)) => send(
            connection,
            Message::Response(Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("unsupported request `{}`", request.method),
            )),
        ),
        Err(ExtractError::JsonError { method, error }) => send(
            connection,
            Message::Response(Response::new_err(
                id,
                ErrorCode::InvalidParams as i32,
                format!("malformed `{method}` request: {error}"),
            )),
        ),
    }
}
