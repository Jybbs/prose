//! The stdin input path: the buffer read off the stream, the name and
//! source type its `--stdin-filename` lends it, the config that name
//! resolves, and the output a stdin run writes back.

use std::io;

use super::{
    process::{Marker, failed, process_source},
    *,
};

/// Formats the buffer `input` holds, writing the formatted source to
/// stdout for a plain text run, the patch under `--diff`, and the records
/// under a structured output format.
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
    let (outcome, original) = match input {
        Ok(text) => (process_stdin(text.clone(), filename, setup, pass), text),
        Err(outcome) => (outcome, String::new()),
    };
    let outcomes = std::slice::from_ref(&outcome);
    let summary = emitter_summary(outcomes);
    // Only a plain text run writes its rewrite out, as the buffer it
    // prints in place of stdin.
    let written = matches!(pass, Pass::Rewrite);
    if let FileOutcome::Done {
        notebook_index,
        rewrite,
        ..
    } = &outcome
    {
        match (pass, rewrite) {
            (Pass::Preview, Rewrite::Changed(kind)) => write_rewrite_diff(
                &mut writer.into_inner(),
                &stdin_name(filename),
                &original,
                kind,
                notebook_index.as_deref(),
                Heading::from(present),
            )?,
            (Pass::Preview, _) => {}
            (Pass::Rewrite, _) => {
                let to_write: &[u8] = match rewrite {
                    Rewrite::Changed(kind) => kind.written().as_bytes(),
                    // A passed-over or unchanged source echoes stdin verbatim.
                    Rewrite::PassedOver | Rewrite::Skipped | Rewrite::Unchanged => {
                        original.as_bytes()
                    }
                };
                writer
                    .into_inner()
                    .write_all(to_write)
                    .context("writing stdout")?;
            }
            (Pass::Both | Pass::Diagnose { .. }, _) => {
                emit_to_stdout(outcomes, common.output_format, present, writer, &summary)?;
            }
        }
    }
    let status = close_run(outcomes, &summary, setup, present, pass, written, stderr);
    // A plain run whose stdout holds the formatted buffer exits clean
    // whatever lint it found.
    Ok(if written && matches!(outcome, FileOutcome::Done { .. }) {
        ExitStatus::Clean
    } else {
        status
    })
}

/// Processes stdin's `text` under the config of the file `filename`
/// names, with its ancestors and overrides, or under the working
/// directory's for an unnamed buffer. A named file whose config fails
/// to load fails at the config-error status, as its path run does.
pub(super) fn process_stdin(
    text: String,
    filename: Option<&Path>,
    setup: &RunSetup,
    pass: Pass,
) -> FileOutcome {
    let resolved = filename.map_or_else(
        || Some(Arc::clone(&setup.cwd)),
        |path| setup.resolver.resolve(path, text.as_bytes()),
    );
    let Some(resolved) = resolved else {
        return FileOutcome::Failed(ExitStatus::ConfigError);
    };
    process_source(
        text,
        stdin_name(filename),
        stdin_source_type(filename),
        &resolved,
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
