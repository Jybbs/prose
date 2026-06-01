//! Runs the pipeline over a tracked buffer and resolves per-document
//! configuration, bridging prose's engine to the editor protocol.

use std::{path::PathBuf, str::FromStr};

use lsp_types::{Diagnostic as LspDiagnostic, TextEdit, Uri};
use ruff_source_file::PositionEncoding;

use super::conversion::{full_document_range, to_lsp};
use crate::{config::Config, pipeline::Pipeline, source::Source};

/// Lints and format-checks the tracked buffer, mapping each finding to a
/// protocol diagnostic against the buffer's own coordinates. An
/// unparseable buffer publishes nothing, clearing any stale findings.
pub(super) fn diagnostics(uri: &Uri, text: &str, encoding: PositionEncoding) -> Vec<LspDiagnostic> {
    let config = config_for(uri);
    let Ok(source) = Source::from_str(text) else {
        return Vec::new();
    };
    Pipeline::with_defaults(&config)
        .diagnose(&source)
        .iter()
        .map(|diagnostic| to_lsp(&source, diagnostic, encoding))
        .collect()
}

/// Formats the tracked buffer and returns one whole-document edit, or
/// `None` when the buffer is already formatted or does not parse.
pub(super) fn format_edits(
    uri: &Uri,
    original: &str,
    encoding: PositionEncoding,
) -> Option<Vec<TextEdit>> {
    let config = config_for(uri);
    let source = Source::from_str(original).ok()?;
    let range = full_document_range(&source, encoding);
    let (formatted, _) = Pipeline::with_defaults(&config).run(source).ok()?;
    (formatted.text() != original).then(|| {
        vec![TextEdit {
            new_text: formatted.text().to_owned(),
            range,
        }]
    })
}

/// Resolves the `[tool.prose]` configuration governing `uri` the way the
/// CLI does, walking up from the document's path. Falls back to defaults
/// for unsaved buffers whose URI carries no filesystem path.
fn config_for(uri: &Uri) -> Config {
    file_path(uri)
        .and_then(|path| Config::load(path).ok())
        .unwrap_or_default()
}

/// Turns a `file://` URI into a filesystem path, or `None` for a URI
/// that names no local file.
fn file_path(uri: &Uri) -> Option<PathBuf> {
    url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use lsp_types::{DiagnosticSeverity, NumberOrString, Position};

    use super::*;

    fn uri(s: &str) -> Uri {
        Uri::from_str(s).expect("valid uri")
    }

    #[test]
    fn config_for_falls_back_to_default_for_unsaved_buffer() {
        let resolved = config_for(&uri("untitled:Untitled-1"));
        assert_eq!(
            resolved.code_line_length,
            Config::default().code_line_length
        );
    }

    #[test]
    fn config_for_reads_prose_toml_beside_the_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("prose.toml"), "code-line-length = 100\n").expect("writes");
        let file = dir.path().join("mod.py");
        let uri = Uri::from_str(&format!("file://{}", file.display())).expect("valid uri");

        let resolved = config_for(&uri);

        assert_eq!(
            resolved.code_line_length.map(std::num::NonZeroUsize::get),
            Some(100),
        );
    }

    #[test]
    fn diagnostics_are_empty_for_clean_source() {
        assert!(diagnostics(&uri("file:///a.py"), "x = 1\n", PositionEncoding::Utf16).is_empty());
    }

    #[test]
    fn diagnostics_are_empty_for_unparseable_source() {
        assert!(diagnostics(&uri("file:///a.py"), "def foo(", PositionEncoding::Utf16).is_empty());
    }

    #[test]
    fn diagnostics_surface_a_lint_finding_as_a_warning() {
        let published = diagnostics(&uri("file:///a.py"), "import os\n", PositionEncoding::Utf16);
        let only = published.first().expect("one diagnostic");

        assert_eq!(only.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(only.source.as_deref(), Some("prose"));
        assert_matches!(&only.code, Some(NumberOrString::String(slug)) if slug == "bare-imports");
    }

    #[test]
    fn diagnostics_surface_format_drift_as_information() {
        let published = diagnostics(
            &uri("file:///a.py"),
            "alpha = 1\nb = 22\n",
            PositionEncoding::Utf16,
        );
        assert!(
            published
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::INFORMATION)),
        );
    }

    #[test]
    fn format_edits_are_none_for_formatted_source() {
        assert!(format_edits(&uri("file:///a.py"), "x = 1\n", PositionEncoding::Utf16).is_none());
    }

    #[test]
    fn format_edits_are_none_for_unparseable_source() {
        assert!(format_edits(&uri("file:///a.py"), "def foo(", PositionEncoding::Utf16).is_none());
    }

    #[test]
    fn format_edits_replace_the_whole_document() {
        let edits = format_edits(
            &uri("file:///a.py"),
            "alpha = 1\nb = 22\n",
            PositionEncoding::Utf16,
        )
        .expect("formatting changes the buffer");
        let edit = edits.first().expect("one edit");

        assert_eq!(
            edit.range.start,
            Position {
                character: 0,
                line: 0
            }
        );
        assert_eq!(
            edit.range.end,
            Position {
                character: 0,
                line: 2
            }
        );
        assert!(edit.new_text.contains("alpha = 1"));
    }
}
