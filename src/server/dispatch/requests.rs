//! LSP request routing: the `textDocument/formatting` handler.

//! Protocol handshake and the synchronous message loop that routes
//! requests and notifications to the formatting and diagnostic passes.

use lsp_server::{Connection, ErrorCode, ExtractError, Message, Request, Response};
use lsp_types::{
    DocumentFormattingParams,
    request::{Formatting, Request as RequestTrait},
};
use ruff_source_file::PositionEncoding;

use super::send;
use crate::server::{analysis, config_cache::ConfigCache, documents::DocumentStore};
/// Routes one request, answering formatting and rejecting any other
/// method so the client never blocks waiting for a response.
pub(super) fn handle_request(
    connection: &Connection,
    documents: &DocumentStore,
    configs: &mut ConfigCache,
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
            let config = configs.resolve(uri);
            let edits = documents
                .get(uri)
                .and_then(|doc| analysis::format_edits(&doc.text, encoding, config));
            send(connection, Message::Response(Response::new_ok(id, edits)))
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
