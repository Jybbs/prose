//! Protocol handshake and the synchronous message loop that routes
//! requests and notifications to the formatting and diagnostic passes.

use anyhow::Context;
use lsp_server::{Connection, ExtractError, Message, Notification, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, InitializeParams, InitializeResult, PublishDiagnosticsParams,
    ServerInfo, Uri,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as NotificationTrait, PublishDiagnostics,
    },
    request::{Formatting, Request as RequestTrait},
};
use ruff_source_file::PositionEncoding;
use serde::de::DeserializeOwned;

use super::{analysis, capabilities, documents::DocumentStore};

/// JSON-RPC code for a request whose method the server does not serve.
const METHOD_NOT_FOUND: i32 = -32601;

/// Completes the `initialize` handshake, negotiating position encoding
/// from the client's capabilities before advertising the server's, then
/// runs the message loop until shutdown.
///
/// Takes `connection` by value so its sender drops when the loop
/// returns. The stdio writer thread runs until every sender drops, so a
/// surviving borrow would deadlock the caller's `io_threads.join()`.
pub(crate) fn serve(connection: Connection) -> anyhow::Result<()> {
    let (id, params) = connection
        .initialize_start()
        .context("starting language-server handshake")?;
    let params: InitializeParams =
        serde_json::from_value(params).context("decoding initialize params")?;
    let encoding = capabilities::negotiate_encoding(&params.capabilities);
    let result = InitializeResult {
        capabilities: capabilities::server_capabilities(encoding),
        server_info: Some(ServerInfo {
            name: "prose".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    let result = serde_json::to_value(result).context("encoding server capabilities")?;
    connection
        .initialize_finish(id, result)
        .context("finishing language-server handshake")?;
    main_loop(&connection, encoding)
}

/// Deserializes a notification's params, contextualizing a decode
/// failure with the method that carried the malformed payload.
fn decode<P: DeserializeOwned>(notification: Notification) -> anyhow::Result<P> {
    serde_json::from_value(notification.params)
        .with_context(|| format!("decoding `{}` params", notification.method))
}

/// Routes one notification by method, updating the document store and
/// republishing the affected document's diagnostics. An open or change
/// replaces the buffer, a close drops it. Unknown methods are ignored
/// (the protocol leaves notifications unanswered), and malformed params
/// surface as an error.
fn handle_notification(
    connection: &Connection,
    documents: &mut DocumentStore,
    notification: Notification,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    let uri = if notification.method == DidOpenTextDocument::METHOD {
        let params: DidOpenTextDocumentParams = decode(notification)?;
        documents.set(params.text_document.uri.clone(), params.text_document.text);
        params.text_document.uri
    } else if notification.method == DidChangeTextDocument::METHOD {
        let mut params: DidChangeTextDocumentParams = decode(notification)?;
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.pop() {
            documents.set(uri.clone(), change.text);
        }
        uri
    } else if notification.method == DidCloseTextDocument::METHOD {
        let params: DidCloseTextDocumentParams = decode(notification)?;
        documents.remove(&params.text_document.uri);
        params.text_document.uri
    } else {
        return Ok(());
    };
    publish(connection, documents, &uri, encoding)
}

/// Routes one request, answering formatting and rejecting any other
/// method so the client never blocks waiting for a response.
fn handle_request(
    connection: &Connection,
    documents: &DocumentStore,
    request: Request,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    match request.extract::<DocumentFormattingParams>(Formatting::METHOD) {
        Ok((id, params)) => {
            let uri = &params.text_document.uri;
            let edits = documents
                .get(uri)
                .and_then(|text| analysis::format_edits(uri, text, encoding));
            send(connection, Message::Response(Response::new_ok(id, edits)))
        }
        Err(ExtractError::MethodMismatch(request)) => send(
            connection,
            Message::Response(Response::new_err(
                request.id,
                METHOD_NOT_FOUND,
                format!("unsupported request `{}`", request.method),
            )),
        ),
        Err(ExtractError::JsonError { method, error }) => {
            Err(anyhow::anyhow!("malformed `{method}` request: {error}"))
        }
    }
}

/// Reads each message until the client requests shutdown.
fn main_loop(connection: &Connection, encoding: PositionEncoding) -> anyhow::Result<()> {
    let mut documents = DocumentStore::default();
    for message in &connection.receiver {
        match message {
            Message::Notification(notification) => {
                handle_notification(connection, &mut documents, notification, encoding)?;
            }
            Message::Request(request) => {
                if connection
                    .handle_shutdown(&request)
                    .context("handling shutdown")?
                {
                    return Ok(());
                }
                handle_request(connection, &documents, request, encoding)?;
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// Recomputes and publishes the tracked buffer's diagnostics, sending an
/// empty list when no buffer is tracked so the editor clears stale marks.
fn publish(
    connection: &Connection,
    documents: &DocumentStore,
    uri: &Uri,
    encoding: PositionEncoding,
) -> anyhow::Result<()> {
    let diagnostics = documents
        .get(uri)
        .map(|text| analysis::diagnostics(uri, text, encoding))
        .unwrap_or_default();
    let params = PublishDiagnosticsParams {
        diagnostics,
        uri: uri.clone(),
        version: None,
    };
    send(
        connection,
        Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            params,
        )),
    )
}

/// Sends one message to the client, contextualizing a closed channel.
fn send(connection: &Connection, message: Message) -> anyhow::Result<()> {
    connection
        .sender
        .send(message)
        .context("sending message to client")
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, thread};

    use lsp_server::RequestId;
    use lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DocumentFormattingParams, FormattingOptions, HoverParams, InitializeParams,
        InitializeResult, InitializedParams, Position, PublishDiagnosticsParams,
        TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, TextEdit, Uri, VersionedTextDocumentIdentifier,
        WorkDoneProgressParams,
    };
    use rstest::rstest;
    use serde::Serialize;
    use serde_json::Value;

    use super::*;

    const DID_CHANGE: &str = "textDocument/didChange";
    const DID_CLOSE: &str = "textDocument/didClose";
    const DID_OPEN: &str = "textDocument/didOpen";
    const EXIT: &str = "exit";
    const FORMATTING: &str = "textDocument/formatting";
    const HOVER: &str = "textDocument/hover";
    const INITIALIZE: &str = "initialize";
    const INITIALIZED: &str = "initialized";
    const PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
    const SHUTDOWN: &str = "shutdown";

    fn uri() -> Uri {
        Uri::from_str("file:///module.py").expect("valid uri")
    }

    fn note<P: Serialize>(method: &str, params: P) -> Message {
        Message::Notification(Notification::new(method.to_owned(), params))
    }

    fn req<P: Serialize>(id: i32, method: &str, params: P) -> Message {
        Message::Request(Request::new(RequestId::from(id), method.to_owned(), params))
    }

    fn recv(client: &Connection) -> Message {
        client
            .receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("server replies within the timeout")
    }

    /// Drives initialize + initialized against `client`, returning the
    /// decoded capabilities so each test starts from a live session.
    fn handshake(client: &Connection) -> InitializeResult {
        client
            .sender
            .send(req(1, INITIALIZE, InitializeParams::default()))
            .expect("send initialize");
        let Message::Response(init) = recv(client) else {
            panic!("expected initialize response");
        };
        let result =
            serde_json::from_value(init.result.expect("initialize result")).expect("decodes");
        client
            .sender
            .send(note(INITIALIZED, InitializedParams {}))
            .expect("send initialized");
        result
    }

    /// Sends shutdown + exit and joins the server thread cleanly.
    fn teardown(client: &Connection, handle: thread::JoinHandle<anyhow::Result<()>>) {
        client
            .sender
            .send(req(3, SHUTDOWN, ()))
            .expect("send shutdown");
        let _ = recv(client);
        client.sender.send(note(EXIT, ())).expect("send exit");
        handle
            .join()
            .expect("server thread joins")
            .expect("serve succeeds");
    }

    fn did_open(client: &Connection, text: &str) {
        client
            .sender
            .send(note(
                DID_OPEN,
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri(),
                        language_id: "python".to_owned(),
                        version: 1,
                        text: text.to_owned(),
                    },
                },
            ))
            .expect("send didOpen");
    }

    fn formatting_request(client: &Connection) -> Vec<TextEdit> {
        client
            .sender
            .send(req(
                2,
                FORMATTING,
                DocumentFormattingParams {
                    text_document: TextDocumentIdentifier { uri: uri() },
                    options: FormattingOptions::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                },
            ))
            .expect("send formatting");
        let Message::Response(response) = recv(client) else {
            panic!("expected formatting response");
        };
        serde_json::from_value::<Option<Vec<TextEdit>>>(response.result.expect("formatting result"))
            .expect("decodes")
            .unwrap_or_default()
    }

    fn published(client: &Connection) -> PublishDiagnosticsParams {
        let Message::Notification(notification) = recv(client) else {
            panic!("expected publishDiagnostics");
        };
        assert_eq!(notification.method, PUBLISH_DIAGNOSTICS);
        serde_json::from_value(notification.params).expect("decodes")
    }

    #[test]
    fn initialize_advertises_the_formatting_provider() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        let result = handshake(&client);
        assert!(result.capabilities.document_formatting_provider.is_some());
        teardown(&client, handle);
    }

    #[test]
    fn did_open_publishes_a_lint_diagnostic() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, "import os\n");
        let params = published(&client);
        assert_eq!(params.diagnostics.len(), 1);
        assert_eq!(params.diagnostics[0].source.as_deref(), Some("prose"));
        teardown(&client, handle);
    }

    #[rstest]
    #[case("import os\n", None)]
    #[case("alpha = 1\nb = 22\n", Some("alpha = 1"))]
    fn formatting_matches_the_buffer_state(#[case] source: &str, #[case] expected: Option<&str>) {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, source);
        let _ = published(&client);
        let edits = formatting_request(&client);
        match expected {
            None => assert!(edits.is_empty(), "formatted buffer needs no edits"),
            Some(needle) => {
                assert_eq!(edits.len(), 1);
                assert!(edits[0].new_text.contains(needle));
            }
        }
        teardown(&client, handle);
    }

    #[test]
    fn unsupported_request_receives_method_not_found() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client
            .sender
            .send(req(
                2,
                HOVER,
                HoverParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri() },
                        position: Position::default(),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                },
            ))
            .expect("send hover");
        let Message::Response(response) = recv(&client) else {
            panic!("expected error response");
        };
        assert_eq!(response.error.expect("error").code, METHOD_NOT_FOUND);
        teardown(&client, handle);
    }

    #[test]
    fn did_change_republishes_against_the_new_text() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, "x = 1\n");
        assert!(published(&client).diagnostics.is_empty());
        client
            .sender
            .send(note(
                DID_CHANGE,
                DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri(),
                        version: 2,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "import os\n".to_owned(),
                    }],
                },
            ))
            .expect("send didChange");
        assert_eq!(published(&client).diagnostics.len(), 1);
        teardown(&client, handle);
    }

    #[test]
    fn did_close_clears_published_diagnostics() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, "import os\n");
        let _ = published(&client);
        client
            .sender
            .send(note(
                DID_CLOSE,
                DidCloseTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri: uri() },
                },
            ))
            .expect("send didClose");
        assert!(published(&client).diagnostics.is_empty());
        teardown(&client, handle);
    }

    #[test]
    fn formatting_an_untracked_document_returns_no_edits() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client
            .sender
            .send(req(
                2,
                FORMATTING,
                DocumentFormattingParams {
                    text_document: TextDocumentIdentifier { uri: uri() },
                    options: FormattingOptions::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                },
            ))
            .expect("send formatting");
        let Message::Response(response) = recv(&client) else {
            panic!("expected formatting response");
        };
        assert_eq!(response.result, Some(Value::Null));
        teardown(&client, handle);
    }
}
