//! `prose config-schema` subcommand: the configuration's JSON Schema.

use std::io::Write;

use schemars::schema_for;

use super::exit_status::ExitStatus;
use crate::config::Config;

/// Prints the JSON Schema derived from [`Config`], pretty-printed.
pub(crate) fn print<W: Write>(mut stdout: W) -> anyhow::Result<ExitStatus> {
    serde_json::to_writer_pretty(&mut stdout, &schema_for!(Config))?;
    writeln!(stdout)?;
    Ok(ExitStatus::Clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render() -> String {
        let mut out = Vec::new();
        print(&mut out).expect("schema emission succeeds");
        String::from_utf8(out).expect("utf8 output")
    }

    #[test]
    fn output_ends_with_a_newline() {
        assert!(render().ends_with('\n'));
    }

    #[test]
    fn output_is_a_json_schema_object() {
        let schema: serde_json::Value = serde_json::from_str(&render()).expect("valid JSON");
        assert!(schema["$schema"].as_str().is_some());
        assert!(schema["properties"].is_object());
    }
}
