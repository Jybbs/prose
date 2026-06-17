//! LSP notification routing: decode and dispatch document open /
//! change / close and watched-file events into republished diagnostics.

use anyhow::Context;
use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    PublishDiagnosticsParams, Uri,
    notification::{
        DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
};
use ruff_source_file::PositionEncoding;
use serde::de::DeserializeOwned;

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
    if notification.method == DidChangeWatchedFiles::METHOD {
        configs.clear();
        return republish_all(connection, documents, configs, encoding);
    }
    let uri = if notification.method == DidOpenTextDocument::METHOD {
        let params: DidOpenTextDocumentParams = decode(notification)?;
        let version = params.text_document.version;
        documents.set(
            params.text_document.uri.clone(),
            params.text_document.text,
            version,
        );
        params.text_document.uri
    } else if notification.method == DidChangeTextDocument::METHOD {
        let mut params: DidChangeTextDocumentParams = decode(notification)?;
        let version = params.text_document.version;
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.pop() {
            documents.set(uri.clone(), change.text, version);
        }
        uri
    } else if notification.method == DidCloseTextDocument::METHOD {
        let params: DidCloseTextDocumentParams = decode(notification)?;
        documents.remove(&params.text_document.uri);
        params.text_document.uri
    } else {
        return Ok(());
    };
    publish(connection, documents, configs, &uri, encoding)
}

/// Deserializes a notification's params, contextualizing a decode
/// failure with the method that carried the malformed payload.
fn decode<P: DeserializeOwned>(notification: Notification) -> anyhow::Result<P> {
    serde_json::from_value(notification.params)
        .with_context(|| format!("decoding `{}` params", notification.method))
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
