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
    let notification = match notification.extract(DidChangeWatchedFiles::METHOD) {
        Ok(DidChangeWatchedFilesParams { .. }) => {
            configs.clear();
            return republish_all(connection, documents, configs, encoding);
        }
        Err(ExtractError::MethodMismatch(notification)) => notification,
        Err(error) => return Err(error.into()),
    };
    let notification = match notification.extract(DidOpenTextDocument::METHOD) {
        Ok(DidOpenTextDocumentParams { text_document }) => {
            documents.set(
                text_document.uri.clone(),
                text_document.text,
                text_document.version,
            );
            return publish(connection, documents, configs, &text_document.uri, encoding);
        }
        Err(ExtractError::MethodMismatch(notification)) => notification,
        Err(error) => return Err(error.into()),
    };
    let notification = match notification.extract(DidChangeTextDocument::METHOD) {
        Ok(DidChangeTextDocumentParams {
            text_document,
            mut content_changes,
        }) => {
            if let Some(change) = content_changes.pop() {
                documents.set(
                    text_document.uri.clone(),
                    change.text,
                    text_document.version,
                );
            }
            return publish(connection, documents, configs, &text_document.uri, encoding);
        }
        Err(ExtractError::MethodMismatch(notification)) => notification,
        Err(error) => return Err(error.into()),
    };
    match notification.extract(DidCloseTextDocument::METHOD) {
        Ok(DidCloseTextDocumentParams { text_document }) => {
            documents.remove(&text_document.uri);
            publish(connection, documents, configs, &text_document.uri, encoding)
        }
        Err(ExtractError::MethodMismatch(_)) => Ok(()),
        Err(error) => Err(error.into()),
    }
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
