//! Github emitter: workflow-command annotations.

use std::io::{self, Write};

use crate::diagnostics::{Diagnostic, Emitter, Run};
use crate::source::Source;

pub(crate) struct Github;

impl Emitter for Github {
    fn emit(&self, writer: &mut dyn Write, runs: &[Run<'_>]) -> io::Result<()> {
        for (source, diagnostics) in runs {
            for diag in *diagnostics {
                emit_one(writer, source, diag)?;
            }
        }
        Ok(())
    }
}

fn emit_one(writer: &mut dyn Write, source: &Source, diag: &Diagnostic) -> io::Result<()> {
    debug_assert!(
        !diag.message.contains(['%', '\r', '\n']),
        "rule message must not carry workflow-command escape characters",
    );
    let start = source.line_column(diag.range.start());
    let end = source.line_column(diag.range.end());
    let file = source.filename();
    let message = diag.message.as_str();
    write!(
        writer,
        "::warning file={file},line={l},col={c}",
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
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::rule::RuleId;

    fn diag(range: TextRange) -> Diagnostic {
        Diagnostic {
            fix: Some(Edit::range_replacement("y".to_owned(), range)),
            message: "rewrite x to y".to_owned(),
            range,
            rule: RuleId::from("rewrite-x"),
            severity: Severity::Format,
        }
    }

    #[test]
    fn emits_endline_and_endcolumn_when_range_stays_on_one_line() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let diag = diag(TextRange::new(0.into(), 1.into()));
        let mut buf = Vec::<u8>::new();
        Github
            .emit(&mut buf, &[(&source, std::slice::from_ref(&diag))])
            .expect("emits");
        assert_eq!(
            String::from_utf8(buf).expect("utf-8"),
            "::warning file=<source>,line=1,col=1,endLine=1,endColumn=2::rewrite x to y\n",
        );
    }

    #[test]
    fn drops_endline_and_endcolumn_for_multi_line_ranges() {
        let source: Source = "x = (\n  1\n)\n".parse().expect("parses");
        let diag = diag(TextRange::new(0.into(), 11.into()));
        let mut buf = Vec::<u8>::new();
        Github
            .emit(&mut buf, &[(&source, std::slice::from_ref(&diag))])
            .expect("emits");
        assert_eq!(
            String::from_utf8(buf).expect("utf-8"),
            "::warning file=<source>,line=1,col=1::rewrite x to y\n",
        );
    }
}
