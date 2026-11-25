//! Tests of the `completions`, `rules`, `schema`, and `server` subcommands.

use crate::{json, prose, stdout_utf8};

#[test]
fn completions_bash_exits_zero() {
    prose().args(["completions", "bash"]).assert().success();
}

#[test]
fn rules_json_lists_every_registered_rule_in_pipeline_order() {
    let assert = prose()
        .args(["rules", "--output-format", "json"])
        .assert()
        .success();

    let rules = json(stdout_utf8(&assert));

    insta::assert_snapshot!(serde_json::to_string_pretty(&rules).expect("renders"));
}

#[test]
fn schema_subcommand_exits_zero_and_prints_the_schema() {
    let assert = prose().arg("schema").assert().success();

    assert!(json(stdout_utf8(&assert))["properties"]["rules"].is_object());
}

#[test]
fn server_completes_a_stdio_session_over_the_real_binary() {
    let session = lsp_session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"untitled:m.py","languageId":"python","version":1,"text":"import os\nos.getcwd()\n"}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":"untitled:m.py"},"options":{"tabSize":4,"insertSpaces":true}}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#,
        r#"{"jsonrpc":"2.0","method":"exit","params":null}"#,
    ]);

    let assert = prose()
        .arg("server")
        .write_stdin(session)
        .assert()
        .success();

    let out = stdout_utf8(&assert);
    assert!(
        out.contains("documentFormattingProvider"),
        "initialize result missing capabilities: {out:?}",
    );
    assert!(
        out.contains("publishDiagnostics") && out.contains("bare-imports"),
        "diagnostics not published: {out:?}",
    );
}

/// Frames each JSON-RPC body with its `Content-Length` header and joins
/// them into one stdio stream the language server can read end to end.
fn lsp_session(bodies: &[&str]) -> String {
    bodies
        .iter()
        .map(|body| format!("Content-Length: {}\r\n\r\n{body}", body.len()))
        .collect()
}
