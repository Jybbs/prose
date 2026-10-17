//! Runs the pipeline over a tracked buffer, bridging prose's engine to the
//! editor protocol.

use std::collections::BTreeSet;

use lsp_types::{Diagnostic as LspDiagnostic, TextEdit};
use ruff_source_file::PositionEncoding;

use super::conversion::{full_document_range, to_lsp};
use crate::{
    config::Config, pipeline::Pipeline, rules::RuleId, source::Source, unstable::UnstableRewrite,
};

/// One formatting pass over a buffer.
#[derive(Default)]
pub(super) struct Formatted {
    /// The whole-document edit, `None` where the buffer is already
    /// formatted or fails to read.
    pub(super) edits: Option<Vec<TextEdit>>,
    /// The rewritten buffer held for a settle check the caller runs
    /// after the edits response is sent, `None` where nothing changed.
    pub(super) settled: Option<Settled>,
}

/// A rewrite held beside the pipeline that produced it and the rules
/// that edited on the way, read by a later settle check.
pub(super) struct Settled {
    fired: BTreeSet<RuleId>,
    pipeline: Pipeline,
    source: Source,
}

impl Settled {
    /// Runs the settle check over the held rewrite, re-applying the
    /// rules that fired as a `format` run does.
    pub(super) fn detect(&self, config: &Config, original: &str) -> Option<UnstableRewrite> {
        UnstableRewrite::detect_narrowed(
            &self.pipeline,
            config,
            original,
            &self.source,
            &self.fired,
        )
    }
}

/// Lints and format-checks the buffer against `config`, mapping each
/// finding to a protocol diagnostic in the buffer's own coordinates. A
/// buffer that fails to read yields nothing, clearing any stale
/// findings.
pub(super) fn diagnostics(
    text: &str,
    encoding: PositionEncoding,
    config: &Config,
) -> Vec<LspDiagnostic> {
    let pipeline = Pipeline::with_defaults(config);
    let Ok(source) = pipeline.read(text) else {
        return Vec::new();
    };
    pipeline
        .diagnose(&source)
        .iter()
        .map(|diagnostic| to_lsp(&source, diagnostic, encoding))
        .collect()
}

/// Formats the buffer against `config`, returning one whole-document
/// edit alongside the rewrite held for a later settle check. A buffer
/// that is already formatted or fails to read yields neither.
pub(super) fn format_buffer(
    original: &str,
    encoding: PositionEncoding,
    config: &Config,
) -> Formatted {
    formatted(original, encoding, config).unwrap_or_default()
}

/// Returns the edit and held rewrite for a buffer the pipeline
/// rewrote, `None` where it fails to read, a rule's output is rejected,
/// or the run leaves the text unchanged.
fn formatted(original: &str, encoding: PositionEncoding, config: &Config) -> Option<Formatted> {
    let pipeline = Pipeline::with_defaults(config);
    let source = pipeline.read(original).ok()?;
    let range = full_document_range(&source, encoding);
    let (settled, _, fired) = pipeline.run(source).ok()?;
    let new_text = settled.changed_from(original)?.to_owned();
    Some(Formatted {
        edits: Some(vec![TextEdit { new_text, range }]),
        settled: Some(Settled {
            fired,
            pipeline,
            source: settled,
        }),
    })
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use lsp_types::{DiagnosticSeverity, NumberOrString, Position};
    use ruff_python_ast::PythonVersion;

    use super::*;
    use crate::testing::BARE_IMPORT_LINT;

    #[test]
    fn diagnostics_are_empty_for_a_gated_import_below_its_version() {
        let config = Config {
            target_version: Some(PythonVersion::PY314),
            ..Config::default()
        };
        assert!(diagnostics("lazy import os\n", PositionEncoding::Utf16, &config).is_empty());
    }

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
        let published = diagnostics(
            BARE_IMPORT_LINT,
            PositionEncoding::Utf16,
            &Config::default(),
        );
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
    fn format_buffer_holds_the_settle_report_for_a_settled_rewrite() {
        let formatted = format_buffer(
            "alpha = 1\nb = 22\n",
            PositionEncoding::Utf16,
            &Config::default(),
        );

        let settled = formatted.settled.expect("a rewrite is held");
        assert!(
            settled
                .detect(&Config::default(), "alpha = 1\nb = 22\n")
                .is_none()
        );
    }

    #[test]
    fn format_buffer_replaces_the_whole_document() {
        let edits = format_buffer(
            "alpha = 1\nb = 22\n",
            PositionEncoding::Utf16,
            &Config::default(),
        )
        .edits
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

    #[test]
    fn format_buffer_yields_no_edits_for_formatted_source() {
        let formatted = format_buffer("x = 1\n", PositionEncoding::Utf16, &Config::default());

        assert!(formatted.edits.is_none());
    }

    #[test]
    fn format_buffer_yields_no_edits_for_unparseable_source() {
        let formatted = format_buffer("def foo(", PositionEncoding::Utf16, &Config::default());

        assert!(formatted.edits.is_none());
    }
}
