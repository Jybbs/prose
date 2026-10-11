//! The pipeline passes a CLI mode reads from a file, and the closing
//! summary each pass resolves into.

use crate::{cache::Anchor, cli::args::OutputFormat};

/// Which closing summary an outcome set resolves into.
#[derive(Clone, Copy)]
pub(super) enum Mode {
    Check,
    Preview,
    Reformat,
}

/// Which pipeline passes a CLI mode reads from a file, and the
/// invocation shape that follows from it. Every per-run flag the runner
/// needs derives from this one value.
#[derive(Clone, Copy, Debug)]
pub(super) enum Pass {
    /// `format` with a `json`, `sarif`, or `github` output format: the
    /// as-written diagnostics and the rewrite, committed to the file.
    Both,
    /// `check`: the as-written diagnostics, no rewrite. `validate` adds
    /// the opt-in guard that rejects a rule whose output fails to
    /// re-parse or to compile.
    Diagnose { validate: bool },
    /// `format --diff`: the rewrite alone, rendered as a patch and left
    /// uncommitted.
    Preview,
    /// Plain `format`: the rewrite alone, committed to the file.
    Rewrite,
}

impl Pass {
    /// Resolves which pipeline passes a `format` invocation reads. A
    /// `--diff` previews the rewrite without committing it, a plain text
    /// rewrite commits it, and a `json`, `sarif`, or `github` output format
    /// also renders the as-written diagnostics beside the commit.
    pub(super) fn for_format(diff: bool, format: OutputFormat) -> Self {
        if diff {
            Self::Preview
        } else if format.is_text() {
            Self::Rewrite
        } else {
            Self::Both
        }
    }

    /// Which buffer this pass's diagnostics resolve against, keying its
    /// entries apart from a pass that reads the other one.
    pub(super) fn anchor(self) -> Anchor {
        match self {
            Self::Both | Self::Diagnose { .. } => Anchor::AsWritten,
            Self::Preview | Self::Rewrite => Anchor::Rewritten,
        }
    }

    /// True where the pass reads and writes the run's cache.
    pub(super) fn caches(self) -> bool {
        // A `--validate` check alone bypasses the cache, so it always
        // re-confirms the rewrite parses.
        !matches!(self, Self::Diagnose { validate: true })
    }

    /// True where the pass never reached a structured emitter, leaving
    /// the surviving-lint count to the closing stderr disclosure.
    pub(super) fn discloses_lint(self) -> bool {
        matches!(self, Self::Preview | Self::Rewrite)
    }

    /// Which closing summary this pass's outcomes resolve into.
    pub(super) fn mode(self) -> Mode {
        match self {
            Self::Both | Self::Rewrite => Mode::Reformat,
            Self::Diagnose { .. } => Mode::Check,
            Self::Preview => Mode::Preview,
        }
    }

    /// The name of the subcommand this pass runs under.
    pub(super) fn subcommand(self) -> &'static str {
        match self {
            Self::Both | Self::Preview | Self::Rewrite => "format",
            Self::Diagnose { .. } => "check",
        }
    }

    /// True where the pass commits its rewrite to the file. The exit
    /// status reads this too, reporting a landed rewrite as clean.
    pub(super) fn write_back(self) -> bool {
        matches!(self, Self::Both | Self::Rewrite)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Pass::Both, Anchor::AsWritten)]
    #[case(Pass::Diagnose { validate: false }, Anchor::AsWritten)]
    #[case(Pass::Diagnose { validate: true }, Anchor::AsWritten)]
    #[case(Pass::Preview, Anchor::Rewritten)]
    #[case(Pass::Rewrite, Anchor::Rewritten)]
    fn anchors_only_a_rewriting_pass_against_the_output(
        #[case] pass: Pass,
        #[case] anchor: Anchor,
    ) {
        assert_eq!(pass.anchor(), anchor);
    }

    #[rstest]
    #[case(Pass::Both, true)]
    #[case(Pass::Diagnose { validate: false }, true)]
    #[case(Pass::Diagnose { validate: true }, false)]
    #[case(Pass::Preview, true)]
    #[case(Pass::Rewrite, true)]
    fn caches_every_shape_but_a_validated_check(#[case] pass: Pass, #[case] caches: bool) {
        assert_eq!(pass.caches(), caches);
    }

    #[rstest]
    #[case(Pass::Both, false)]
    #[case(Pass::Diagnose { validate: false }, false)]
    #[case(Pass::Preview, true)]
    #[case(Pass::Rewrite, true)]
    fn discloses_lint_only_where_no_emitter_printed_it(
        #[case] pass: Pass,
        #[case] discloses: bool,
    ) {
        assert_eq!(pass.discloses_lint(), discloses);
    }

    #[test]
    fn for_format_needs_both_only_for_a_diagnostic_format() {
        assert_matches!(Pass::for_format(false, OutputFormat::Json), Pass::Both);
        assert_matches!(Pass::for_format(false, OutputFormat::Text), Pass::Rewrite);
        assert_matches!(Pass::for_format(true, OutputFormat::Json), Pass::Preview);
        assert_matches!(Pass::for_format(true, OutputFormat::Text), Pass::Preview);
    }

    #[rstest]
    #[case(Pass::Both, Mode::Reformat)]
    #[case(Pass::Diagnose { validate: false }, Mode::Check)]
    #[case(Pass::Preview, Mode::Preview)]
    #[case(Pass::Rewrite, Mode::Reformat)]
    fn resolves_its_closing_summary_mode(#[case] pass: Pass, #[case] expected: Mode) {
        assert_eq!(
            std::mem::discriminant(&pass.mode()),
            std::mem::discriminant(&expected),
        );
    }

    #[rstest]
    #[case(Pass::Both, "format")]
    #[case(Pass::Diagnose { validate: false }, "check")]
    #[case(Pass::Diagnose { validate: true }, "check")]
    #[case(Pass::Preview, "format")]
    #[case(Pass::Rewrite, "format")]
    fn runs_under_its_subcommand(#[case] pass: Pass, #[case] subcommand: &str) {
        assert_eq!(pass.subcommand(), subcommand);
    }

    #[rstest]
    #[case(Pass::Both, true)]
    #[case(Pass::Diagnose { validate: false }, false)]
    #[case(Pass::Preview, false)]
    #[case(Pass::Rewrite, true)]
    fn writes_back_only_where_it_commits_the_rewrite(#[case] pass: Pass, #[case] write_back: bool) {
        assert_eq!(pass.write_back(), write_back);
    }
}
