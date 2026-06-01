//! Runs the pipeline over a tracked buffer, bridging prose's engine to the
//! editor protocol.

use std::str::FromStr;

use lsp_types::{Diagnostic as LspDiagnostic, TextEdit};
use ruff_source_file::PositionEncoding;

use super::conversion::{full_document_range, to_lsp};
use crate::{config::Config, pipeline::Pipeline, source::Source};

/// Lints and format-checks the buffer against `config`, mapping each
/// finding to a protocol diagnostic in the buffer's own coordinates. An
/// unparseable buffer yields nothing, clearing any stale findings.
pub(super) fn diagnostics(
    text: &str,
    encoding: PositionEncoding,
    config: &Config,
) -> Vec<LspDiagnostic> {
    let Ok(source) = Source::from_str(text) else {
        return Vec::new();
    };
    Pipeline::with_defaults(config)
        .diagnose(&source)
        .iter()
        .map(|diagnostic| to_lsp(&source, diagnostic, encoding))
        .collect()
}

/// Formats the buffer against `config` and returns one whole-document
/// edit, or `None` when the buffer is already formatted or does not parse.
pub(super) fn format_edits(
    original: &str,
    encoding: PositionEncoding,
    config: &Config,
) -> Option<Vec<TextEdit>> {
    let source = Source::from_str(original).ok()?;
    let range = full_document_range(&source, encoding);
    let (formatted, _) = Pipeline::with_defaults(config).run(source).ok()?;
    (formatted.text() != original).then(|| {
        vec![TextEdit {
            new_text: formatted.text().to_owned(),
            range,
        }]
    })
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use lsp_types::{DiagnosticSeverity, NumberOrString, Position};

    use super::*;

    #[test]
    fn diagnostics_are_empty_for_clean_source() {
        assert!(diagnostics("x = 1\n", PositionEncoding::Utf16, &Config::default()).is_empty());
    }

    #[test]
    fn diagnostics_are_empty_for_unparseable_source() {
        assert!(diagnostics("def foo(", PositionEncoding::Utf16, &Config::default()).is_empty());
    }

    #[test]
    fn diagnostics_surface_a_lint_finding_as_a_warning() {
        let published = diagnostics("import os\n", PositionEncoding::Utf16, &Config::default());
        let only = published.first().expect("one diagnostic");

        assert_eq!(only.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(only.source.as_deref(), Some("prose"));
        assert_matches!(&only.code, Some(NumberOrString::String(slug)) if slug == "bare-imports");
    }

    #[test]
    fn diagnostics_surface_format_drift_as_information() {
        let published = diagnostics(
            "alpha = 1\nb = 22\n",
            PositionEncoding::Utf16,
            &Config::default(),
        );
        assert!(
            published
                .iter()
                .any(|d| d.severity == Some(DiagnosticSeverity::INFORMATION)),
        );
    }

    #[test]
    fn format_edits_are_none_for_formatted_source() {
        assert!(format_edits("x = 1\n", PositionEncoding::Utf16, &Config::default()).is_none());
    }

    #[test]
    fn format_edits_are_none_for_unparseable_source() {
        assert!(format_edits("def foo(", PositionEncoding::Utf16, &Config::default()).is_none());
    }

    #[test]
    fn format_edits_replace_the_whole_document() {
        let edits = format_edits(
            "alpha = 1\nb = 22\n",
            PositionEncoding::Utf16,
            &Config::default(),
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
