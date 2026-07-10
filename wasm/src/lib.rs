//! WebAssembly bindings exposing the formatting core to JavaScript.

use std::error::Error;

use prose::{config::Config, pipeline::Pipeline, source::Source};
use wasm_bindgen::prelude::*;

/// The output of [`format`]: the rewritten source and the effective
/// configuration that produced it, serialized to TOML.
#[derive(Debug)]
#[wasm_bindgen(getter_with_clone)]
pub struct FormatResult {
    pub config: String,
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

/// Installs the hook that forwards panic messages to the console.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Runs the pipeline behind [`format`], boxing whichever error arises.
fn try_format(config_toml: &str, source: &str) -> Result<FormatResult, Box<dyn Error>> {
    let config = Config::from_prose_toml_str(config_toml)?;
    let (formatted, _diagnostics) =
        Pipeline::with_defaults(&config).run(source.parse::<Source>()?)?;
    Ok(FormatResult {
        config: config.to_toml(),
        formatted: formatted.text().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn formats_an_empty_source() {
        let Ok(result) = format("", "") else {
            panic!("format succeeds");
        };
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
    fn reports_the_effective_config() {
        let Ok(result) = format("code-line-length = 100", "x = 1\n") else {
            panic!("format succeeds");
        };
        assert!(result.config.contains("code-line-length = 100"));
    }

    #[test]
    fn rewrites_the_source() {
        let Ok(result) = format("", "import b\nimport a\n") else {
            panic!("format succeeds");
        };
        assert_eq!(result.formatted, "import a\nimport b\n");
    }

    #[test]
    fn tolerates_an_unknown_config_key() {
        assert_matches!(try_format("no-such-key = 1", "x = 1\n"), Ok(_));
    }
}
