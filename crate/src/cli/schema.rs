//! `prose schema` subcommand: the configuration's JSON Schema.

use std::io::{self, Write};

use schemars::schema_for;

use super::exit_status::ExitStatus;
use crate::config::Config;

/// Prints the JSON Schema derived from [`Config`], pretty-printed.
pub(crate) fn print<W: Write>(mut stdout: W) -> anyhow::Result<ExitStatus> {
    serde_json::to_writer_pretty(&mut stdout, &schema_for!(Config)).map_err(io::Error::from)?;
    writeln!(stdout)?;
    Ok(ExitStatus::Clean)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FailingWriter;

    fn render() -> String {
        let mut out = Vec::new();
        print(&mut out).expect("schema emission succeeds");
        String::from_utf8(out).expect("utf8 output")
    }

    #[test]
    fn broken_pipe_write_finalizes_to_the_clean_exit() {
        let result = print(FailingWriter(io::ErrorKind::BrokenPipe));
        assert_eq!(crate::cli::finalize(result), ExitStatus::Clean);
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
