//! LSP notification routing: decode and dispatch document open /
//! change / close and watched-file events into republished diagnostics.

use lsp_server::{Connection, ExtractError, Message, Notification};
use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, PublishDiagnosticsParams, Uri,
    notification::{
        DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
};
use ruff_source_file::PositionEncoding;

use super::send;
use crate::server::{analysis, config_cache::ConfigCache, documents::DocumentStore};

/// `notification`'s params where its method is `N`'s, and otherwise the
/// notification handed back for the next method to read.
fn extracted<N: NotificationTrait>(
    notification: Notification,
) -> anyhow::Result<Result<N::Params, Notification>> {
    match notification.extract(N::METHOD) {
        Ok(params) => Ok(Ok(params)),
        Err(ExtractError::MethodMismatch(notification)) => Ok(Err(notification)),
        Err(error) => Err(error.into()),
    }
}

/// Routes one notification by method, updating the document store and
/// republishing the affected document's diagnostics. An open or change
/// replaces the buffer, a close drops it. Unknown methods are ignored
/// (the protocol leaves notifications unanswered), and malformed params
/// surface as an error.
pub(super) fn handle_notification(
    connection: &Connection,
    documents: &mut DocumentStore,
    configs: &mut ConfigCache,
    notification: Notification,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    let notification = match extracted::<DidChangeWatchedFiles>(notification)? {
        Ok(DidChangeWatchedFilesParams { .. }) => {
            configs.clear();
            return republish_all(connection, documents, configs, encoding);
        }
        Err(notification) => notification,
    };
    let notification = match extracted::<DidOpenTextDocument>(notification)? {
        Ok(DidOpenTextDocumentParams { text_document }) => {
            documents.set(
                text_document.uri.clone(),
                text_document.text,
                text_document.version,
            );
            return publish(connection, documents, configs, &text_document.uri, encoding);
        }
        Err(notification) => notification,
    };
    let notification = match extracted::<DidChangeTextDocument>(notification)? {
        Ok(DidChangeTextDocumentParams {
            text_document,
            mut content_changes,
        }) => {
            let text = content_changes.pop().map(|change| change.text);
            documents.update(&text_document.uri, text, text_document.version);
            return publish(connection, documents, configs, &text_document.uri, encoding);
        }
        Err(notification) => notification,
    };
    if let Ok(DidCloseTextDocumentParams { text_document }) =
        extracted::<DidCloseTextDocument>(notification)?
    {
        documents.remove(&text_document.uri);
        return publish(connection, documents, configs, &text_document.uri, encoding);
    }
    Ok(())
}

/// Recomputes and publishes the tracked buffer's diagnostics, sending an
/// empty list when no buffer is tracked so the editor clears stale marks.
fn publish(
    connection: &Connection,
    documents: &DocumentStore,
    configs: &mut ConfigCache,
    uri: &Uri,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    let doc = documents.get(uri);
    let diagnostics = doc
        .map(|doc| {
            let config = configs.resolve(uri, &doc.text);
            analysis::diagnostics(&doc.text, encoding, &config)
        })
        .unwrap_or_default();
    let params = PublishDiagnosticsParams {
        diagnostics,
        uri: uri.clone(),
        version: doc.map(|doc| doc.version),
    };
    send(
        connection,
        Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            params,
        )),
    )
}

/// Recomputes and republishes diagnostics for every open buffer, after a
/// config change invalidates their cached settings.
fn republish_all(
    connection: &Connection,
    documents: &DocumentStore,
    configs: &mut ConfigCache,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    for uri in documents.uris() {
        publish(connection, documents, configs, &uri, encoding)?;
    }
    Ok(())
}
