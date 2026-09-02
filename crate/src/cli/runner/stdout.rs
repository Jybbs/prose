//! Diagnostic emission to the process stdout, one emitter per output
//! format.

use std::io::{BufWriter, Write};

use anstream::{
    AutoStream,
    stream::{AsLockedWrite, RawStream},
};
use anyhow::Context;

use super::FileOutcome;
use crate::cli::{
    args::OutputFormat,
    emit::{Emitter, EmitterSummary, Github, Json, Run, Sarif, Text},
    output::Presentation,
};

/// Emits `outcomes` to the process stdout `stdout` wraps.
///
/// A structured format writes to the raw stream through a `BufWriter`,
/// since json, sarif and github hold no escape sequence for the
/// `AutoStream` to strip and each emits many small writes. Text keeps
/// the `AutoStream`, which `--color always` needs, and writes blocks
/// large enough that a second buffer buys nothing.
pub(super) fn emit_to_stdout<O: RawStream + AsLockedWrite>(
    outcomes: &[FileOutcome],
    format: OutputFormat,
    present: &Presentation,
    stdout: AutoStream<O>,
    summary: &EmitterSummary,
) -> anyhow::Result<()> {
    if format.is_text() {
        let mut stdout = stdout;
        emit_outcomes(outcomes, format, present, &mut stdout, summary)
    } else {
        let mut buffered = BufWriter::new(stdout.into_inner());
        emit_outcomes(outcomes, format, present, &mut buffered, summary)
    }
}

/// The text block `outcome` renders to, empty for an outcome the
/// report leaves out, which is the same set
/// [`emit_outcomes`](self::emit_outcomes) filters away.
pub(super) fn render_text_block(text: &Text, outcome: &FileOutcome) -> anyhow::Result<Vec<u8>> {
    outcome.run().map_or_else(
        || Ok(Vec::new()),
        |run| text.render_run(&run).context("rendering diagnostics"),
    )
}

fn emit_outcomes<W: Write>(
    outcomes: &[FileOutcome],
    format: OutputFormat,
    present: &Presentation,
    writer: &mut W,
    summary: &EmitterSummary,
) -> anyhow::Result<()> {
    let view: Vec<Run<'_>> = outcomes.iter().filter_map(FileOutcome::run).collect();
    match format {
        OutputFormat::Github => Github.emit(writer, &view, summary),
        OutputFormat::Json => Json.emit(writer, &view, summary),
        OutputFormat::Sarif => Sarif.emit(writer, &view, summary),
        OutputFormat::Text => Text::new(present.color).emit(writer, &view, summary),
    }?;
    writer.flush().context("flushing stdout")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{assert_matches, io};

    use rstest::rstest;

    use super::{
        super::tests::{diagnostic, outcome_with},
        *,
    };
    use crate::diagnostics::Severity;

    use crate::testing::{FailingWriter, parse, range};

    #[test]
    fn emit_outcomes_propagates_the_writer_failure_kind() {
        let source = parse("x = 1\n");
        let diags = vec![diagnostic(
            Severity::Format,
            range(0, 1),
            "synthetic-format",
        )];
        let outcomes = vec![outcome_with(source, diags)];
        let result = emit_outcomes(
            &outcomes,
            OutputFormat::Json,
            &Presentation::windowed(),
            &mut FailingWriter(io::ErrorKind::BrokenPipe),
            &EmitterSummary::default(),
        );
        let err = result.expect_err("writer failure propagates");
        assert_matches!(
            err.downcast_ref::<io::Error>(),
            Some(e) if e.kind() == io::ErrorKind::BrokenPipe
        );
    }

    #[rstest]
    fn emit_outcomes_renders_each_output_format(
        #[values(
            OutputFormat::Github,
            OutputFormat::Json,
            OutputFormat::Sarif,
            OutputFormat::Text
        )]
        format: OutputFormat,
    ) {
        let source = parse("x = 1\n");
        let outcomes = vec![outcome_with(source, Vec::new())];
        let mut buf = Vec::new();
        emit_outcomes(
            &outcomes,
            format,
            &Presentation::windowed(),
            &mut buf,
            &EmitterSummary::default(),
        )
        .expect("emits");
    }
}
