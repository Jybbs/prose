//! Protocol handshake and the synchronous message loop that routes
//! requests and notifications to the formatting and diagnostic passes.

use anyhow::Context;
use lsp_server::{Connection, Message, Request, RequestId};
use lsp_types::{
    ClientCapabilities, DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern,
    InitializeParams, InitializeResult, Registration, RegistrationParams, ServerInfo,
    notification::{DidChangeWatchedFiles, Exit, Notification as NotificationTrait},
    request::{RegisterCapability, Request as RequestTrait},
};
use ruff_source_file::PositionEncoding;

use super::{capabilities, config_cache::ConfigCache, documents::DocumentStore};
use crate::config::config_rel_paths;

mod notifications;
mod requests;

use notifications::handle_notification;
use requests::handle_request;

/// Sends one message to the client, contextualizing a closed channel.
pub(super) fn send(connection: &Connection, message: Message) -> anyhow::Result<()> {
    connection
        .sender
        .send(message)
        .context("sending message to client")
}

/// Completes the `initialize` handshake, negotiating position encoding
/// from the client's capabilities before advertising the server's, then
/// runs the message loop until shutdown.
///
/// Takes `connection` by value so its sender drops when the loop
/// returns. The stdio writer thread runs until every sender drops, so a
/// surviving borrow would deadlock the caller's `io_threads.join()`.
pub(super) fn serve(connection: Connection) -> anyhow::Result<()> {
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
    let watching = register_config_watchers(&connection, &params.capabilities)?;
    main_loop(&connection, encoding, watching)
}

/// Reads each message until the client requests shutdown or sends a bare
/// `exit`. A malformed message is logged and dropped rather than ending the
/// session, so one bad payload never tears down a live editor.
fn main_loop(
    connection: &Connection,
    encoding: PositionEncoding,
    watching: bool,
) -> anyhow::Result<()> {
    let mut documents = DocumentStore::default();
    let mut configs = ConfigCache::new(watching);
    for message in &connection.receiver {
        match message {
            Message::Notification(notification) => {
                if notification.method == Exit::METHOD {
                    return Ok(());
                }
                if let Err(err) = handle_notification(
                    connection,
                    &mut documents,
                    &mut configs,
                    notification,
                    encoding,
                ) {
                    eprintln!("prose server: dropped notification: {err:#}");
                }
            }
            Message::Request(request) => match connection.handle_shutdown(&request) {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    if let Err(err) =
                        handle_request(connection, &documents, &mut configs, request, encoding)
                    {
                        eprintln!("prose server: request failed: {err:#}");
                    }
                }
                Err(err) => {
                    eprintln!("prose server: shutdown handshake failed: {err}");
                    return Ok(());
                }
            },
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// Registers a `workspace/didChangeWatchedFiles` watcher for prose's config
/// files when the client supports dynamic registration, so editing any of
/// them refreshes every open buffer. Clients without dynamic registration
/// still pick up config changes on the next edit.
fn register_config_watchers(
    connection: &Connection,
    capabilities: &ClientCapabilities,
) -> anyhow::Result<bool> {
    let supported = capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.did_change_watched_files.as_ref())
        .and_then(|watched| watched.dynamic_registration)
        == Some(true);
    if !supported {
        return Ok(false);
    }
    let options = DidChangeWatchedFilesRegistrationOptions {
        watchers: config_rel_paths()
            .into_iter()
            .map(|path| FileSystemWatcher {
                glob_pattern: GlobPattern::String(format!("**/{path}")),
                kind: None,
            })
            .collect(),
    };
    let params = RegistrationParams {
        registrations: vec![Registration {
            id: "prose-config-watch".to_owned(),
            method: DidChangeWatchedFiles::METHOD.to_owned(),
            register_options: Some(
                serde_json::to_value(options).context("encoding watcher registration")?,
            ),
        }],
    };
    send(
        connection,
        Message::Request(Request::new(
            RequestId::from("prose/register-config-watch".to_owned()),
            RegisterCapability::METHOD.to_owned(),
            params,
        )),
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::thread;

    use lsp_server::{ErrorCode, Notification, RequestId, Response};
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
    use crate::testing;

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
        testing::uri("file:///module.py")
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
    fn bare_exit_without_shutdown_ends_the_loop() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client.sender.send(note(EXIT, ())).expect("send exit");
        handle
            .join()
            .expect("server thread joins")
            .expect("serve returns Ok on bare exit");
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
                        text: "import os\nos.getcwd()\n".to_owned(),
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
        did_open(&client, "import os\nos.getcwd()\n");
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
    fn did_open_publishes_a_lint_diagnostic() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, "import os\nos.getcwd()\n");
        let params = published(&client);
        assert_eq!(params.diagnostics.len(), 1);
        assert_eq!(params.diagnostics[0].source.as_deref(), Some("prose"));
        teardown(&client, handle);
    }

    #[test]
    fn dynamic_registration_capable_client_gets_a_watcher_registration() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        client
            .sender
            .send(req(
                1,
                INITIALIZE,
                serde_json::json!({
                    "capabilities": {
                        "workspace": {
                            "didChangeWatchedFiles": { "dynamicRegistration": true }
                        }
                    }
                }),
            ))
            .expect("send initialize");
        let Message::Response(_) = recv(&client) else {
            panic!("expected initialize response");
        };
        client
            .sender
            .send(note(INITIALIZED, InitializedParams {}))
            .expect("send initialized");
        let Message::Request(registration) = recv(&client) else {
            panic!("expected registerCapability request");
        };
        assert_eq!(registration.method, "client/registerCapability");
        let params: RegistrationParams = serde_json::from_value(registration.params.clone())
            .expect("decodes registration params");
        let options: DidChangeWatchedFilesRegistrationOptions = serde_json::from_value(
            params.registrations[0]
                .register_options
                .clone()
                .expect("watcher options"),
        )
        .expect("decodes watcher options");
        let globs: Vec<String> = options
            .watchers
            .into_iter()
            .map(|watcher| match watcher.glob_pattern {
                GlobPattern::String(glob) => glob,
                GlobPattern::Relative(_) => unreachable!("prose registers string globs"),
            })
            .collect();
        assert!(globs.contains(&"**/.config/prose.toml".to_owned()));
        client
            .sender
            .send(Message::Response(Response::new_ok(
                registration.id,
                serde_json::Value::Null,
            )))
            .expect("ack registration");
        client.sender.send(note(EXIT, ())).expect("send exit");
        handle
            .join()
            .expect("server thread joins")
            .expect("serve returns Ok");
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
    fn initialize_advertises_the_formatting_provider() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        let result = handshake(&client);
        assert!(result.capabilities.document_formatting_provider.is_some());
        teardown(&client, handle);
    }

    #[test]
    fn loop_returns_ok_when_the_client_disconnects() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        drop(client);
        handle
            .join()
            .expect("server thread joins")
            .expect("serve returns Ok on disconnect");
    }

    #[test]
    fn malformed_notification_is_dropped_and_server_survives() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client
            .sender
            .send(note(DID_OPEN, serde_json::json!({ "bogus": true })))
            .expect("send malformed didOpen");
        did_open(&client, "import os\nos.getcwd()\n");
        assert_eq!(published(&client).diagnostics.len(), 1);
        teardown(&client, handle);
    }

    #[test]
    fn malformed_request_receives_invalid_params() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client
            .sender
            .send(req(2, FORMATTING, serde_json::json!({ "bogus": true })))
            .expect("send malformed formatting");
        let Message::Response(response) = recv(&client) else {
            panic!("expected error response");
        };
        assert_eq!(
            response.error.expect("error").code,
            ErrorCode::InvalidParams as i32
        );
        teardown(&client, handle);
    }

    #[test]
    fn non_exit_after_shutdown_ends_the_session_cleanly() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client
            .sender
            .send(req(3, SHUTDOWN, ()))
            .expect("send shutdown");
        client
            .sender
            .send(note("textDocument/didSave", serde_json::json!({})))
            .expect("send a non-exit message");
        handle
            .join()
            .expect("server thread joins")
            .expect("serve returns Ok despite the protocol violation");
    }

    #[test]
    fn published_diagnostics_carry_the_document_version() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, "import os\nos.getcwd()\n");
        assert_eq!(published(&client).version, Some(1));
        teardown(&client, handle);
    }

    #[test]
    fn unknown_notification_is_ignored() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        client
            .sender
            .send(note("textDocument/didSave", serde_json::json!({})))
            .expect("send unknown notification");
        did_open(&client, "import os\nos.getcwd()\n");
        assert_eq!(published(&client).diagnostics.len(), 1);
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
        assert_eq!(
            response.error.expect("error").code,
            ErrorCode::MethodNotFound as i32
        );
        teardown(&client, handle);
    }

    #[test]
    fn watched_file_change_republishes_open_documents() {
        let (server, client) = Connection::memory();
        let handle = thread::spawn(move || serve(server));
        handshake(&client);
        did_open(&client, "import os\nos.getcwd()\n");
        assert_eq!(published(&client).diagnostics.len(), 1);
        client
            .sender
            .send(note(
                "workspace/didChangeWatchedFiles",
                serde_json::json!({ "changes": [] }),
            ))
            .expect("send didChangeWatchedFiles");
        assert_eq!(published(&client).diagnostics.len(), 1);
        teardown(&client, handle);
    }
}
