//! Github emitter: workflow-command annotations.

use std::io::{self, Write};

use ruff_notebook::NotebookIndex;
use ruff_source_file::SourceFile;

use super::{Emitter, EmitterSummary, Run, diagnostics};
use crate::{
    diagnostics::Diagnostic,
    findings::{cell_message, located},
    rules::render_slugs,
};

pub(crate) struct Github;

impl Emitter for Github {
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        summary: &EmitterSummary,
    ) -> io::Result<()> {
        for (file, index, diag) in diagnostics(runs) {
            emit_one(writer, file, index, diag)?;
        }
        for entry in &summary.unstable {
            writeln!(
                writer,
                "::warning file={}::prose produced output a second run would change ({})",
                escape_property(&entry.file),
                render_slugs(&entry.rules),
            )?;
        }
        Ok(())
    }
}

fn emit_one(
    writer: &mut dyn Write,
    file: &SourceFile,
    index: Option<&NotebookIndex>,
    diag: &Diagnostic,
) -> io::Result<()> {
    let (start, end, cell) = located(file, index, diag.range);
    let name = escape_property(file.name());
    let message = escape_data(&cell_message(&diag.message, cell));
    write!(
        writer,
        "::warning file={name},line={l},col={c}",
        l = start.line,
        c = start.column,
    )?;
    if start.line == end.line {
        write!(
            writer,
            ",endLine={el},endColumn={ec}",
            el = end.line,
            ec = end.column,
        )?;
    }
    writeln!(writer, "::{message}")
}

/// Escapes each `%`, carriage return, and newline in a workflow command's
/// message, the mapping `@actions/core`'s `escapeData` applies and the
/// runner reverses.
fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escapes a workflow command's property value as `escape_data` escapes a
/// message, and each `:` and `,` besides, the mapping `@actions/core`'s
/// `escapeProperty` applies.
fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use ruff_source_file::SourceFileBuilder;

    use super::*;
    use crate::{
        cli::emit::{UnstableEntry, emitted_runs, emitted_string},
        rules::RuleId,
        testing::{FailingWriter, format_diagnostic, parse, range},
    };

    fn emit_to_string(file: &SourceFile, diag: &Diagnostic) -> String {
        emitted_string(
            &Github,
            file,
            std::slice::from_ref(diag),
            &EmitterSummary::default(),
        )
    }

    #[test]
    fn drops_endline_and_endcolumn_for_multi_line_ranges() {
        let source = parse("x = (\n  1\n)\n");
        let diag = format_diagnostic(range(0, 11));
        assert_eq!(
            emit_to_string(source.source_file(), &diag),
            "::warning file=<source>,line=1,col=1::rewrite x to y\n",
        );
    }

    #[test]
    fn emits_endline_and_endcolumn_when_range_stays_on_one_line() {
        let source = parse("x = 1\n");
        let diag = format_diagnostic(range(0, 1));
        assert_eq!(
            emit_to_string(source.source_file(), &diag),
            "::warning file=<source>,line=1,col=1,endLine=1,endColumn=2::rewrite x to y\n",
        );
    }

    #[test]
    fn escapes_the_file_name_and_the_message_of_a_diagnostic() {
        let file = SourceFileBuilder::new("a,b%c:d.py", "x = 1\n").finish();
        let diag = Diagnostic {
            message: "first\r\nsecond at 100%".to_owned(),
            ..format_diagnostic(range(0, 1))
        };
        assert_eq!(
            emit_to_string(&file, &diag),
            "::warning file=a%2Cb%25c%3Ad.py,line=1,col=1,endLine=1,endColumn=2::first%0D%0Asecond at 100%25\n",
        );
    }

    #[test]
    fn escapes_the_name_of_a_file_a_second_run_would_change() {
        let source = parse("x = 1\n");
        let summary = EmitterSummary {
            unstable: vec![UnstableEntry {
                file: "a,b.py".to_owned(),
                rules: vec![RuleId::from("align-equals")],
            }],
            ..EmitterSummary::default()
        };
        let runs = [Run::new(source.source_file(), &[], None)];
        let emitted = emitted_runs(&Github, &runs, &summary);
        assert_eq!(
            String::from_utf8(emitted).expect("utf-8"),
            "::warning file=a%2Cb.py::prose produced output a second run would change (`align-equals`)\n",
        );
    }

    #[test]
    fn propagates_the_error_a_failing_writer_raises() {
        let source = parse("x = 1\n");
        let diag = format_diagnostic(range(0, 1));
        let runs = [Run::new(
            source.source_file(),
            std::slice::from_ref(&diag),
            None,
        )];
        let result = Github.emit(
            &mut FailingWriter(io::ErrorKind::BrokenPipe),
            &runs,
            &EmitterSummary::default(),
        );
        assert_matches!(result, Err(e) if e.kind() == io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn warns_for_each_file_a_second_run_would_change() {
        let source = parse("x = 1\n");
        let summary = EmitterSummary {
            unstable: vec![UnstableEntry {
                file: "a.py".to_owned(),
                rules: vec![RuleId::from("align-colons"), RuleId::from("align-equals")],
            }],
            ..EmitterSummary::default()
        };
        let runs = [Run::new(source.source_file(), &[], None)];
        let emitted = emitted_runs(&Github, &runs, &summary);
        assert_eq!(
            String::from_utf8(emitted).expect("utf-8"),
            "::warning file=a.py::prose produced output a second run would change (`align-colons`, `align-equals`)\n",
        );
    }
}
