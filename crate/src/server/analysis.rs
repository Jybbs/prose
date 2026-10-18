//! Runs the pipeline over a tracked buffer, bridging prose's engine to the
//! editor protocol.

use std::collections::BTreeSet;

use lsp_types::{Diagnostic as LspDiagnostic, TextEdit};
use ruff_python_ast::PySourceType;
use ruff_source_file::PositionEncoding;

use super::conversion::{full_document_range, refusal_to_lsp, to_lsp};
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

/// Lints and format-checks the buffer, read in `source_type`'s mode,
/// against `config`, mapping each finding to a protocol diagnostic in the
/// buffer's own coordinates. A buffer that does not parse yields
/// nothing, clearing any stale findings, and one holding syntax its
/// target rejects yields that refusal alone as an error.
pub(super) fn diagnostics(
    text: &str,
    source_type: PySourceType,
    encoding: PositionEncoding,
    config: &Config,
) -> Vec<LspDiagnostic> {
    let pipeline = Pipeline::with_defaults(config);
    let Ok(source) = pipeline.read(text, source_type) else {
        return Vec::new();
    };
    if let Some(refusal) = pipeline.refusal(&source) {
        return vec![refusal_to_lsp(
            &source,
            &refusal,
            pipeline.target(),
            encoding,
        )];
    }
    pipeline
        .diagnose(&source)
        .iter()
        .map(|diagnostic| to_lsp(&source, diagnostic, encoding))
        .collect()
}

/// Formats the buffer, read in `source_type`'s mode, against `config`,
/// returning one whole-document edit alongside the rewrite held for a
/// later settle check. A buffer that is already formatted, does not
/// parse, or holds syntax its target rejects yields neither.
pub(super) fn format_buffer(
    original: &str,
    source_type: PySourceType,
    encoding: PositionEncoding,
    config: &Config,
) -> Formatted {
    formatted(original, source_type, encoding, config).unwrap_or_default()
}

/// Returns the edit and held rewrite for a buffer the pipeline
/// rewrote, `None` where it does not parse, holds syntax its target
/// rejects, a rule's output is rejected, or the run leaves the text
/// unchanged.
fn formatted(
    original: &str,
    source_type: PySourceType,
    encoding: PositionEncoding,
    config: &Config,
) -> Option<Formatted> {
    let pipeline = Pipeline::with_defaults(config);
    let source = pipeline
        .read(original, source_type)
        .ok()
        .filter(|source| pipeline.refusal(source).is_none())?;
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
    use rstest::rstest;
    use ruff_python_ast::PythonVersion;

    use super::*;
    use crate::testing::BARE_IMPORT_LINT;

    #[test]
    fn diagnostics_are_empty_for_clean_source() {
        assert!(
            diagnostics(
                "x = 1\n",
                PySourceType::Python,
                PositionEncoding::Utf16,
                &Config::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn diagnostics_are_empty_for_unparseable_source() {
        assert!(
            diagnostics(
                "def foo(",
                PySourceType::Python,
                PositionEncoding::Utf16,
                &Config::default()
            )
            .is_empty()
        );
    }

    #[rstest]
    #[case::a_stub(PySourceType::Stub, 1)]
    #[case::a_module(PySourceType::Python, 0)]
    fn diagnostics_read_a_buffer_in_its_document_mode(
        #[case] source_type: PySourceType,
        #[case] refused: usize,
    ) {
        let config = Config {
            target_version: Some(PythonVersion::PY313),
            ..Config::default()
        };

        let published = diagnostics(
            "def f(x: (y := 1)) -> None: ...\n",
            source_type,
            PositionEncoding::Utf16,
            &config,
        );

        let errors = published
            .iter()
            .filter(|diagnostic| diagnostic.severity == Some(DiagnosticSeverity::ERROR))
            .count();
        assert_eq!(errors, refused);
    }

    #[test]
    fn diagnostics_report_a_refused_buffer_as_one_error() {
        let config = Config {
            target_version: Some(PythonVersion::PY314),
            ..Config::default()
        };

        let published = diagnostics(
            "lazy import os\n",
            PySourceType::Python,
            PositionEncoding::Utf16,
            &config,
        );

        assert_matches!(
            published.as_slice(),
            [LspDiagnostic {
                code: Some(NumberOrString::String(code)),
                severity: Some(DiagnosticSeverity::ERROR),
                ..
            }] if code == "syntax-error"
        );
        assert_eq!(published[0].range.end, Position::new(0, 4));
        assert!(published[0].message.contains("`target-version = \"3.14\"`"));
    }

    #[test]
    fn diagnostics_surface_a_lint_finding_as_a_warning() {
        let published = diagnostics(
            BARE_IMPORT_LINT,
            PySourceType::Python,
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
            PySourceType::Python,
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
            PySourceType::Python,
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
            PySourceType::Python,
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
        let formatted = format_buffer(
            "x = 1\n",
            PySourceType::Python,
            PositionEncoding::Utf16,
            &Config::default(),
        );

        assert!(formatted.edits.is_none());
    }

    #[test]
    fn format_buffer_yields_no_edits_for_unparseable_source() {
        let formatted = format_buffer(
            "def foo(",
            PySourceType::Python,
            PositionEncoding::Utf16,
            &Config::default(),
        );

        assert!(formatted.edits.is_none());
    }
}
