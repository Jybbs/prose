//! The stdin input path: the buffer read off the stream, the name and
//! source type its `--stdin-filename` lends it, the config that name
//! resolves, and the output a stdin run writes back.

use std::io;

use super::{
    process::{Marker, failed, process_source},
    *,
};

pub(super) fn format_stdin<O: RawStream + AsLockedWrite, E: Write>(
    input: Result<String, FileOutcome>,
    common: &RunArgs,
    pass: Pass,
    present: &Presentation,
    setup: &RunSetup,
    writer: AutoStream<O>,
    stderr: &mut E,
) -> anyhow::Result<ExitStatus> {
    let filename = common.stdin_filename.as_deref();
    let format = common.output_format;
    let diff = matches!(pass, Pass::Preview);
    let (outcome, original) = match input {
        Ok(text) => {
            let resolved = stdin_resolved(setup, filename, &text);
            (process_stdin(text.clone(), filename, &resolved, pass), text)
        }
        Err(outcome) => (outcome, String::new()),
    };
    let outcomes = std::slice::from_ref(&outcome);
    let summary = emitter_summary(outcomes);
    if let FileOutcome::Done {
        notebook_index,
        rewrite,
        ..
    } = &outcome
    {
        if diff {
            if let Rewrite::Changed(kind) = rewrite {
                let heading = diff_heading(present);
                write_rewrite_diff(
                    &mut writer.into_inner(),
                    &stdin_name(filename),
                    &original,
                    kind,
                    notebook_index.as_deref(),
                    heading,
                )?;
            }
        } else if format.is_text() {
            let to_write: &[u8] = match rewrite {
                Rewrite::Changed(kind) => kind.written().as_bytes(),
                // A non-Python notebook carries no rewrite, so echo stdin verbatim.
                Rewrite::PassedOver | Rewrite::Skipped | Rewrite::Unchanged => original.as_bytes(),
            };
            writer
                .into_inner()
                .write_all(to_write)
                .context("writing stdout")?;
        } else {
            emit_to_stdout(outcomes, format, present, writer, &summary)?;
        }
    }
    Ok(close_run(outcomes, &summary, setup, present, pass, stderr))
}

/// The resolution governing a stdin run: the config of the file
/// `filename` names, so a named buffer draws the ancestors and
/// overrides its on-disk twin would, and the working directory's for an
/// unnamed one.
pub(super) fn stdin_resolved(
    setup: &RunSetup,
    filename: Option<&Path>,
    text: &str,
) -> Arc<Resolved> {
    filename
        .and_then(|path| setup.resolver.resolve(path, text.as_bytes()))
        .unwrap_or_else(|| Arc::clone(&setup.cwd))
}

/// The name a stdin buffer reports under, the path `filename` names
/// or the placeholder for an unnamed one.
fn stdin_name(filename: Option<&Path>) -> String {
    filename.map_or_else(|| STDIN_NAME.to_owned(), |path| path.display().to_string())
}

/// Resolves the source type of stdin input from a `--stdin-filename`,
/// defaulting to Python when none is given.
fn stdin_source_type(filename: Option<&Path>) -> PySourceType {
    filename
        .and_then(PySourceType::try_from_path)
        .unwrap_or_default()
}

pub(super) fn process_stdin(
    text: String,
    filename: Option<&Path>,
    resolved: &Resolved,
    pass: Pass,
) -> FileOutcome {
    process_source(
        text,
        stdin_name(filename),
        stdin_source_type(filename),
        resolved,
        pass,
        Marker::Eager,
    )
}

/// Reads stdin to a string, mapping a read failure to a config-error
/// outcome.
pub(super) fn read_stdin<R: Read>(stdin: R) -> Result<String, FileOutcome> {
    io::read_to_string(stdin)
        .map_err(|e| failed(ExitStatus::ConfigError, format_args!("reading stdin: {e}")))
}
