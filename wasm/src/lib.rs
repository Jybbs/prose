//! WebAssembly bindings exposing the formatting core to JavaScript.

use std::{collections::BTreeSet, error::Error};

use prose::{
    config::Config,
    diagnostics::Diagnostic,
    findings::{JsonDiagnostic, lint_records},
    pipeline::Pipeline,
    rules::RuleId,
    source::Source,
};
use ruff_text_size::Ranged;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// The record `format` returns, reading its text and slugs out of the
/// `Run` that produced it and building its findings per call.
#[derive(Serialize)]
struct FormatRecord<'a> {
    config_notices: &'a [String],
    diagnostics: Vec<JsonDiagnostic<'a>>,
    fired_rules: &'a BTreeSet<RuleId>,
    formatted: &'a str,
    unstable_rules: &'a [RuleId],
}

/// The result of one pipeline run, owning the data `FormatRecord`
/// borrows.
#[derive(Debug)]
struct Run {
    config_notices: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    fired_rules: BTreeSet<RuleId>,
    formatted: Source,
    unstable_rules: Vec<RuleId>,
}

impl Run {
    fn record(&self) -> FormatRecord<'_> {
        FormatRecord {
            config_notices: &self.config_notices,
            diagnostics: lint_records(self.formatted.source_file(), &self.diagnostics),
            fired_rules: &self.fired_rules,
            formatted: self.formatted.text(),
            unstable_rules: &self.unstable_rules,
        }
    }
}

/// Formats `source` using the `prose.toml` text in `config_toml`.
///
/// With `settle`, a run whose last trip edited re-applies the rules that
/// fired to its output, and any rule still editing it is listed in
/// `unstable_rules`. That list is otherwise empty.
///
/// # Errors
///
/// Throws a `JsError` when `config_toml` is not valid config TOML,
/// when `source` does not read as Python under the configured
/// `target-version`, or when a rule's output is rejected by the
/// reparse, the compile gate, or the batch splice.
#[wasm_bindgen]
pub fn format(config_toml: &str, source: &str, settle: bool) -> Result<JsValue, JsError> {
    // `JsError` construction calls a wasm import that panics on a
    // non-wasm target, so only this shim touches it.
    serialized(config_toml, source, settle).map_err(|error| JsError::new(&error.to_string()))
}

/// Panics unconditionally.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn panic_for_test() {
    panic!("prose_wasm smoke-test panic");
}

/// Runs the pipeline over `source`, walking `unsettled_among` over the
/// fired set where `settle` is true and every trip edited.
fn run(config_toml: &str, source: &str, settle: bool) -> Result<Run, Box<dyn Error>> {
    let (config, config_notices) = Config::from_prose_toml_str(config_toml)?;
    let pipeline = Pipeline::with_defaults(&config);
    let source = pipeline.read(source, Default::default())?;
    if let Some(refusal) = pipeline.refusal(&source) {
        let message = refusal.message(pipeline.target());
        return Err(format!("{message} at byte range {:?}", refusal.range()).into());
    }
    let (formatted, diagnostics, trips) = pipeline.run(source)?;
    let unstable_rules = if settle {
        pipeline.unsettled_among(&formatted, &trips)
    } else {
        Vec::new()
    };
    Ok(Run {
        config_notices,
        diagnostics,
        fired_rules: trips.fired,
        formatted,
        unstable_rules,
    })
}

/// Builds the run and serializes its record, boxing any error.
fn serialized(config_toml: &str, source: &str, settle: bool) -> Result<JsValue, Box<dyn Error>> {
    Ok(serde_wasm_bindgen::to_value(
        &run(config_toml, source, settle)?.record(),
    )?)
}

/// Installs the hook that forwards panic messages to the console.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

    /// Already-formatted source, whose bare `import os` draws one
    /// `bare-imports` finding and no edit.
    const BARE_IMPORT_LINT: &str = "import os\n\nos.getcwd()\n";

    fn formatted(config_toml: &str, source: &str) -> Run {
        run(config_toml, source, true).expect("format succeeds")
    }

    #[test]
    fn formats_an_empty_source() {
        let result = formatted("", "");
        assert_eq!(result.formatted.text(), "");
    }

    #[test]
    fn holds_the_fired_set_with_the_settle_check_off() {
        let probe = run("", "alpha = 1\nb = 22\n", false).expect("format succeeds");
        assert!(probe.unstable_rules.is_empty());
        assert!(!probe.fired_rules.is_empty());
    }

    #[rstest]
    #[case::aligned_by_default("", "aa = 1\nb  = 2\n")]
    #[case::left_alone_when_toggled_off("rules.align-equals = false", "aa = 1\nb = 2\n")]
    fn honors_a_rule_toggle(#[case] config_toml: &str, #[case] expected: &str) {
        assert_eq!(
            formatted(config_toml, "aa = 1\nb = 2\n").formatted.text(),
            expected
        );
    }

    #[rstest]
    #[case::config_notices_for_a_known_key(
        "code-line-length = 100",
        "x = 1\n",
        |r: &Run| r.config_notices.is_empty()
    )]
    #[case::fired_rules_when_none_fire("", "x = 1\n", |r: &Run| r.fired_rules.is_empty())]
    #[case::lint_records_when_none_fire("", "x = 1\n", |r: &Run| r.record().diagnostics.is_empty())]
    #[case::unstable_rules_for_a_settled_rewrite(
        "",
        "alpha = 1\nb = 22\n",
        |r: &Run| r.unstable_rules.is_empty()
    )]
    fn leaves_a_field_empty(
        #[case] config_toml: &str,
        #[case] source: &str,
        #[case] is_empty: fn(&Run) -> bool,
    ) {
        assert!(is_empty(&formatted(config_toml, source)));
    }

    #[rstest]
    #[case::a_lazy_import_below_its_target_version(
        "target-version = \"3.14\"",
        "lazy import os\n",
        " at byte range 0..4"
    )]
    #[case::an_invalid_config("code-line-length = \"wide\"", "x = 1\n", "expected a nonzero usize")]
    #[case::unparseable_python("", "def (\n", "Expected an identifier")]
    fn rejects(#[case] config_toml: &str, #[case] source: &str, #[case] fragment: &str) {
        assert_matches!(
            run(config_toml, source, true),
            Err(error) if error.to_string().contains(fragment),
            "the error names {fragment:?}"
        );
    }

    #[test]
    fn reports_an_unknown_config_key_as_a_notice() {
        let result = formatted("no-such-key = 1", "x = 1\n");
        assert_eq!(
            result.config_notices,
            ["warning: unknown key `no-such-key` in [tool.prose]"]
        );
    }

    #[test]
    fn reports_lint_findings_against_the_output() {
        let result = formatted("", BARE_IMPORT_LINT);
        assert_eq!(result.formatted.text(), BARE_IMPORT_LINT);
        assert!(!result.record().diagnostics.is_empty());
    }

    #[test]
    fn reports_the_rules_that_fired_on_the_source() {
        let result = formatted("", "aa = 1\nb = 2\n");
        assert!(
            result
                .fired_rules
                .iter()
                .any(|rule| rule.as_str() == "align-equals")
        );
    }

    #[test]
    fn rewrites_the_source() {
        let result = formatted("", "import b\nimport a\n\nvalue = a, b\n");
        assert_eq!(
            result.formatted.text(),
            "import a\nimport b\n\nvalue = a, b\n"
        );
    }
}
