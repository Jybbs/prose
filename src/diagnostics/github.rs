//! Github emitter: workflow-command annotations.

use std::io::{self, Write};

use ruff_source_file::SourceFile;

use crate::diagnostics::{line_columns, Diagnostic, Emitter, EmitterSummary, Run};

pub(crate) struct Github;

impl Emitter for Github {
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        _summary: &EmitterSummary,
    ) -> io::Result<()> {
        for (file, diagnostics) in runs {
            for diag in *diagnostics {
                emit_one(writer, file, diag)?;
            }
        }
        Ok(())
    }
}

fn emit_one(writer: &mut dyn Write, file: &SourceFile, diag: &Diagnostic) -> io::Result<()> {
    debug_assert!(
        !diag.message.contains(['%', '\r', '\n']),
        "rule message must not carry workflow-command escape characters",
    );
    let (start, end) = line_columns(file, diag.range);
    let name = file.name();
    let message = diag.message.as_str();
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::RuleId;
    use crate::source::Source;

    fn diag(range: TextRange) -> Diagnostic {
        Diagnostic {
            fix: Some(vec![Edit::range_replacement("y".to_owned(), range)]),
            message: "rewrite x to y".to_owned(),
            range,
            rule: RuleId::from("rewrite-x"),
            severity: Severity::Format,
        }
    }

    fn emit_to_string(file: &SourceFile, diag: &Diagnostic) -> String {
        let mut buf = Vec::<u8>::new();
        Github
            .emit(
                &mut buf,
                &[(file, std::slice::from_ref(diag))],
                &EmitterSummary::default(),
            )
            .expect("emits");
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn drops_endline_and_endcolumn_for_multi_line_ranges() {
        let source: Source = "x = (\n  1\n)\n".parse().expect("parses");
        let diag = diag(TextRange::new(0.into(), 11.into()));
        assert_eq!(
            emit_to_string(source.source_file(), &diag),
            "::warning file=<source>,line=1,col=1::rewrite x to y\n",
        );
    }

    #[test]
    fn emits_endline_and_endcolumn_when_range_stays_on_one_line() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = diag(TextRange::new(0.into(), 1.into()));
        assert_eq!(
            emit_to_string(source.source_file(), &diag),
            "::warning file=<source>,line=1,col=1,endLine=1,endColumn=2::rewrite x to y\n",
        );
    }
}
