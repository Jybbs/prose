//! Session-scoped bug notices for a rewrite a second run would change.
//!
//! A document draws its notice once per session rather than once per
//! save. Where the client advertises `window/showDocument`, the notice
//! carries an action that opens the pre-filled report form, and the
//! reply naming that action is what sends the open request.

use std::collections::{HashMap, HashSet};

use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{
    MessageActionItem, MessageType, ShowDocumentParams, ShowMessageParams,
    ShowMessageRequestParams, Uri,
    notification::{Notification as NotificationTrait, ShowMessage},
    request::{Request as RequestTrait, ShowDocument, ShowMessageRequest},
};

use super::{
    conversion,
    dispatch::{send, send_request},
};
use crate::unstable::{UnstableRewrite, blame, headline, report_url};

/// The action title offered beside a notice.
const FILE_IT: &str = "File a report";

/// Which documents this session has already notified, and the document
/// and form behind each outstanding action.
pub(super) struct Notices {
    pending: HashMap<RequestId, (Uri, Uri)>,
    reported: HashSet<Uri>,
    show_document: bool,
}

impl Notices {
    /// Builds the tracker, with `show_document` set from whether the
    /// client advertised the show-document request.
    pub(super) fn new(show_document: bool) -> Self {
        Self {
            pending: HashMap::new(),
            reported: HashSet::new(),
            show_document,
        }
    }

    /// Sends one outbound request under `id`, recording the document
    /// and its form against it where a reply would open the form.
    fn request<P: serde::Serialize>(
        &mut self,
        connection: &Connection,
        id: RequestId,
        method: &str,
        params: P,
        form: Option<(Uri, Uri)>,
    ) -> anyhow::Result<()> {
        if let Some(form) = form {
            self.pending.insert(id.clone(), form);
        }
        send_request(connection, id, method, params)
    }

    /// Opens the form behind a reply naming the report action, and
    /// drops every other reply.
    ///
    /// # Errors
    ///
    /// Returns whatever `send` returns when the channel to the client
    /// has closed.
    pub(super) fn answered(
        &mut self,
        connection: &Connection,
        response: &Response,
    ) -> anyhow::Result<()> {
        let Some((document, form)) = self.pending.remove(&response.id) else {
            return Ok(());
        };
        let picked = response
            .response_result
            .as_ref()
            .ok()
            .and_then(|value| {
                serde_json::from_value::<Option<MessageActionItem>>(value.clone()).ok()
            })
            .flatten();
        if picked.is_none_or(|action| action.title != FILE_IT) {
            return Ok(());
        }
        self.request(
            connection,
            RequestId::from(format!("prose/form-{}", document.as_str())),
            ShowDocument::METHOD,
            ShowDocumentParams {
                external: Some(true),
                selection: None,
                take_focus: Some(true),
                uri: form,
            },
            None,
        )
    }

    /// Sends `rewrite`'s notice for `uri` the first time the session
    /// reaches it, dropping every later one for the same document.
    ///
    /// # Errors
    ///
    /// Returns whatever `send` returns when the channel to the client
    /// has closed.
    pub(super) fn offer(
        &mut self,
        connection: &Connection,
        uri: &Uri,
        original: &str,
        rewrite: &UnstableRewrite,
    ) -> anyhow::Result<()> {
        if !self.reported.insert(uri.clone()) {
            return Ok(());
        }
        let message = notice(uri, rewrite);
        let url = report_url(rewrite, original);
        let Some(form) = self
            .show_document
            .then(|| url.parse::<Uri>().ok())
            .flatten()
        else {
            return send(
                connection,
                Message::Notification(lsp_server::Notification::new(
                    ShowMessage::METHOD.to_owned(),
                    ShowMessageParams {
                        message: format!("{message} File it at {url}"),
                        typ: MessageType::WARNING,
                    },
                )),
            );
        };
        self.request(
            connection,
            RequestId::from(format!("prose/notice-{}", uri.as_str())),
            ShowMessageRequest::METHOD,
            ShowMessageRequestParams {
                actions: Some(vec![MessageActionItem {
                    properties: HashMap::new(),
                    title: FILE_IT.to_owned(),
                }]),
                message,
                typ: MessageType::WARNING,
            },
            Some((uri.clone(), form)),
        )
    }
}

/// The one-line notice a client renders, naming Prose as the defect's
/// source and the invocation that reproduces it. A buffer naming no
/// local file keeps its URI in the sentence and takes the `-` stdin
/// positional in the invocation.
fn notice(uri: &Uri, rewrite: &UnstableRewrite) -> String {
    let path = conversion::to_path(uri).map(|path| path.display().to_string());
    let name = path.clone().unwrap_or_else(|| uri.as_str().to_owned());
    format!(
        "{}. {}. Reproduce it with `{}`.",
        headline(&name),
        blame(1),
        rewrite.invocation(path.as_deref()),
    )
}

#[cfg(test)]
mod tests {
    use lsp_server::ErrorCode;

    use super::*;
    use crate::server::uri;

    const ORIGINAL: &str = "x = 1\n";

    fn drained(client: &Connection) -> Vec<Message> {
        client.receiver.try_iter().collect()
    }

    fn rewrite() -> UnstableRewrite {
        UnstableRewrite::sample("align-equals")
    }

    #[test]
    fn answered_drops_an_error_reply() {
        let (server, client) = Connection::memory();
        let mut notices = Notices::new(true);
        notices
            .offer(&server, &uri("file:///a.py"), ORIGINAL, &rewrite())
            .expect("offers");
        let Some(Message::Request(request)) = drained(&client).into_iter().next() else {
            panic!("expected a message request");
        };

        notices
            .answered(
                &server,
                &Response::new_err(
                    request.id,
                    ErrorCode::RequestCanceled as i32,
                    "cancelled".to_owned(),
                ),
            )
            .expect("answers");

        assert!(
            drained(&client).is_empty(),
            "an error reply opened a document"
        );
    }

    #[test]
    fn answered_drops_a_reply_naming_no_action() {
        let (server, client) = Connection::memory();
        let mut notices = Notices::new(true);
        notices
            .offer(&server, &uri("file:///a.py"), ORIGINAL, &rewrite())
            .expect("offers");
        let Some(Message::Request(request)) = drained(&client).into_iter().next() else {
            panic!("expected a message request");
        };

        notices
            .answered(
                &server,
                &Response::new_ok(request.id, serde_json::Value::Null),
            )
            .expect("answers");

        assert!(
            drained(&client).is_empty(),
            "a null reply opened a document"
        );
    }

    #[test]
    fn answered_drops_a_reply_to_a_request_it_never_issued() {
        let (server, client) = Connection::memory();

        Notices::new(true)
            .answered(
                &server,
                &Response::new_ok(
                    RequestId::from("prose/register-config-watch".to_owned()),
                    serde_json::Value::Null,
                ),
            )
            .expect("answers");

        assert!(drained(&client).is_empty());
    }

    #[test]
    fn answered_opens_the_form_behind_the_report_action() {
        let (server, client) = Connection::memory();
        let mut notices = Notices::new(true);
        notices
            .offer(&server, &uri("file:///a.py"), ORIGINAL, &rewrite())
            .expect("offers");
        let Some(Message::Request(request)) = drained(&client).into_iter().next() else {
            panic!("expected a message request");
        };

        notices
            .answered(
                &server,
                &Response::new_ok(
                    request.id,
                    MessageActionItem {
                        properties: HashMap::new(),
                        title: FILE_IT.to_owned(),
                    },
                ),
            )
            .expect("answers");

        let Some(Message::Request(opened)) = drained(&client).into_iter().next() else {
            panic!("expected a show-document request");
        };
        assert_eq!(opened.method, ShowDocument::METHOD);
        let params: ShowDocumentParams = serde_json::from_value(opened.params).expect("decodes");
        assert_eq!(params.external, Some(true));
        assert!(params.uri.as_str().contains("issues/new"));
    }

    #[test]
    fn offer_carries_the_url_inline_without_show_document() {
        let (server, client) = Connection::memory();

        Notices::new(false)
            .offer(&server, &uri("file:///a.py"), ORIGINAL, &rewrite())
            .expect("offers");

        let Some(Message::Notification(notification)) = drained(&client).into_iter().next() else {
            panic!("expected a show-message notification");
        };
        assert_eq!(notification.method, ShowMessage::METHOD);
        let params: ShowMessageParams =
            serde_json::from_value(notification.params).expect("decodes");
        assert!(params.message.contains("issues/new"), "{}", params.message);
    }

    #[test]
    fn offer_names_the_reproducing_invocation() {
        let (server, client) = Connection::memory();

        Notices::new(true)
            .offer(&server, &uri("file:///a.py"), ORIGINAL, &rewrite())
            .expect("offers");

        let Some(Message::Request(request)) = drained(&client).into_iter().next() else {
            panic!("expected a message request");
        };
        let params: ShowMessageRequestParams =
            serde_json::from_value(request.params).expect("decodes");
        assert!(
            params
                .message
                .contains("prose format --select align-equals /a.py"),
            "{}",
            params.message,
        );
        assert_eq!(
            params.actions.expect("an action")[0].title,
            FILE_IT.to_owned(),
        );
    }

    #[test]
    fn offer_names_the_stdin_positional_for_a_pathless_buffer() {
        let (server, client) = Connection::memory();

        Notices::new(false)
            .offer(&server, &uri("untitled:Untitled-1"), ORIGINAL, &rewrite())
            .expect("offers");

        let Some(Message::Notification(notification)) = drained(&client).into_iter().next() else {
            panic!("expected a show-message notification");
        };
        let params: ShowMessageParams =
            serde_json::from_value(notification.params).expect("decodes");
        assert!(
            params.message.contains("prose rewrote untitled:Untitled-1"),
            "{}",
            params.message,
        );
        assert!(
            params
                .message
                .contains("prose format --select align-equals -"),
            "{}",
            params.message,
        );
    }

    #[test]
    fn offer_sends_one_notice_per_document_per_session() {
        let (server, client) = Connection::memory();
        let mut notices = Notices::new(true);
        let document = uri("file:///a.py");

        notices
            .offer(&server, &document, ORIGINAL, &rewrite())
            .expect("offers");
        notices
            .offer(&server, &document, ORIGINAL, &rewrite())
            .expect("offers");

        assert_eq!(drained(&client).len(), 1);
    }
}
