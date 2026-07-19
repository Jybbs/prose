//! WebAssembly bindings exposing the formatting core to JavaScript.

use std::{collections::BTreeSet, error::Error};

use prose::{config::Config, findings::lint_records_json, pipeline::Pipeline, source::Source};
use wasm_bindgen::prelude::*;

/// The output of [`format`]: the rewritten source, the effective
/// configuration serialized to TOML, the lint-severity findings as the
/// JSON records the docs site decorates, and the distinct slugs of
/// every rule that fired on the source.
#[derive(Debug)]
#[wasm_bindgen(getter_with_clone)]
pub struct FormatResult {
    pub config: String,
    pub diagnostics: String,
    pub fired_rules: Vec<String>,
    pub formatted: String,
}

/// Formats `source` under the `prose.toml` document in `config_toml`.
///
/// # Errors
///
/// Throws a `JsError` when `config_toml` is not valid config TOML,
/// when `source` does not parse as Python, or when a rule's output
/// fails to re-parse.
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
    let (formatted, diagnostics) =
        Pipeline::with_defaults(&config).run(source.parse::<Source>()?)?;
    let fired: BTreeSet<&str> = diagnostics.iter().map(|diag| diag.rule.as_str()).collect();
    Ok(FormatResult {
        config: config.to_toml(),
        diagnostics: lint_records_json(formatted.source_file(), &diagnostics).unwrap_or_default(),
        fired_rules: fired.into_iter().map(String::from).collect(),
        formatted: formatted.text().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    fn formatted(config_toml: &str, source: &str) -> FormatResult {
        format(config_toml, source).unwrap_or_else(|_| panic!("format succeeds"))
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
        let result =
            try_format("", "import os\nos.getcwd()\n").expect("formats despite a lint finding");
        assert_eq!(result.formatted, "import os\nos.getcwd()\n");
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
        let result = formatted("", "import os\nos.getcwd()\n");
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
        let result = formatted("", "import b\nimport a\n");
        assert_eq!(result.formatted, "import a\nimport b\n");
    }

    #[test]
    fn tolerates_an_unknown_config_key() {
        assert_matches!(try_format("no-such-key = 1", "x = 1\n"), Ok(_));
    }
}
