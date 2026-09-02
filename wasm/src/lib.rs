//! WebAssembly bindings exposing the formatting core to JavaScript.

use std::{collections::BTreeSet, error::Error};

use prose::{
    config::Config, diagnostics::Severity, findings::lint_records_json, pipeline::Pipeline,
    rule::RuleId, source::Source,
};
use wasm_bindgen::prelude::*;

/// The output of [`format`]: the rewritten source, the effective
/// configuration serialized to TOML, the lint-severity findings as the
/// JSON records the docs site decorates, the distinct slugs of every
/// rule that fired on the source, and the slugs of any rule a second
/// run would still edit, which names the output unstable.
#[derive(Debug)]
#[wasm_bindgen(getter_with_clone)]
pub struct FormatResult {
    pub config: String,
    pub diagnostics: String,
    pub fired_rules: Vec<String>,
    pub formatted: String,
    pub unstable_rules: Vec<String>,
}

/// Formats `source` under the `prose.toml` document in `config_toml`.
///
/// # Errors
///
/// Throws a `JsError` when `config_toml` is not valid config TOML,
/// when `source` does not parse as Python, or when a rule's output is
/// rejected by the reparse, the compile gate, or the batch splice.
#[wasm_bindgen]
pub fn format(config_toml: &str, source: &str) -> Result<FormatResult, JsError> {
    // `JsError` construction calls a wasm import that panics on a
    // non-wasm target, so only this shim touches it.
    try_format(config_toml, source).map_err(|error| JsError::new(&error.to_string()))
}

/// Panics unconditionally.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn panic_for_test() {
    panic!("prose_wasm smoke-test panic");
}

/// Installs the hook that forwards panic messages to the console.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Runs the pipeline behind [`format`], boxing whichever error arises.
fn try_format(config_toml: &str, source: &str) -> Result<FormatResult, Box<dyn Error>> {
    let config = Config::from_prose_toml_str(config_toml)?;
    let pipeline = Pipeline::with_defaults(&config);
    let (formatted, diagnostics) = pipeline.run(source.parse::<Source>()?)?;
    let fired: BTreeSet<&str> = diagnostics
        .iter()
        .filter(|diag| diag.severity == Severity::Format)
        .map(|diag| diag.rule.as_str())
        .collect();
    Ok(FormatResult {
        config: config.to_toml(),
        diagnostics: lint_records_json(formatted.source_file(), &diagnostics).unwrap_or_default(),
        fired_rules: fired.into_iter().map(String::from).collect(),
        formatted: formatted.text().to_owned(),
        unstable_rules: pipeline
            .unsettled(&formatted)
            .iter()
            .map(RuleId::as_str)
            .map(String::from)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    /// Formatted module source whose bare `import os` draws one
    /// `bare-imports` finding and no format edit.
    const BARE_IMPORT_LINT: &str = "import os\n\nos.getcwd()\n";

    fn formatted(config_toml: &str, source: &str) -> FormatResult {
        format(config_toml, source).expect("format succeeds")
    }

    #[test]
    fn deduplicates_repeat_rule_firings() {
        let result = formatted("", "aa = 1\nb = 2\n\ncc = 3\nd = 4\n");
        assert_eq!(
            result
                .fired_rules
                .iter()
                .filter(|slug| *slug == "align-equals")
                .count(),
            1
        );
    }

    #[test]
    fn formats_an_empty_source() {
        let result = formatted("", "");
        assert_eq!(result.formatted, "");
    }

    #[test]
    fn honors_a_rule_toggle() {
        let aligned = try_format("", "aa = 1\nb = 2\n").expect("formats");
        assert_eq!(aligned.formatted, "aa = 1\nb  = 2\n");
        let toggled = try_format("rules.align-equals = false", "aa = 1\nb = 2\n").expect("formats");
        assert_eq!(toggled.formatted, "aa = 1\nb = 2\n");
    }

    #[test]
    fn leaves_diagnostics_empty_when_none_fire() {
        let result = formatted("", "x = 1\n");
        assert_eq!(result.diagnostics, "");
    }

    #[test]
    fn leaves_fired_rules_empty_when_none_fire() {
        let result = formatted("", "x = 1\n");
        assert!(result.fired_rules.is_empty());
    }

    #[test]
    fn lint_findings_do_not_error() {
        let result = try_format("", BARE_IMPORT_LINT).expect("formats despite a lint finding");
        assert_eq!(result.formatted, BARE_IMPORT_LINT);
    }

    #[test]
    fn rejects_an_invalid_config() {
        assert_matches!(try_format("code-line-length = \"wide\"", "x = 1\n"), Err(_));
    }

    #[test]
    fn rejects_unparseable_python() {
        assert_matches!(try_format("", "def (\n"), Err(_));
    }

    #[test]
    fn reports_lint_findings_against_the_output() {
        let result = formatted("", BARE_IMPORT_LINT);
        assert!(result.diagnostics.contains("bare-imports"));
    }

    #[test]
    fn reports_the_effective_config() {
        let result = formatted("code-line-length = 100", "x = 1\n");
        assert!(result.config.contains("code-line-length = 100"));
    }

    #[test]
    fn reports_the_rules_that_fired_on_the_source() {
        let result = formatted("", "aa = 1\nb = 2\n");
        assert!(result.fired_rules.iter().any(|slug| slug == "align-equals"));
    }

    #[test]
    fn rewrites_the_source() {
        let result = formatted("", "import b\nimport a\n\nvalue = a, b\n");
        assert_eq!(result.formatted, "import a\nimport b\n\nvalue = a, b\n");
    }

    #[test]
    fn tolerates_an_unknown_config_key() {
        assert_matches!(try_format("no-such-key = 1", "x = 1\n"), Ok(_));
    }
}
