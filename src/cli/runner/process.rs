//! Per-file processing: read, cache-lookup, run the pipeline, and
//! classify the outcome.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use rayon::iter::{ParallelBridge, ParallelIterator};
use ruff_source_file::SourceFileBuilder;

use super::{FileOutcome, Pass, RunSetup, has_format_change};
use crate::cli::exit_status::ExitStatus;
use crate::{
    cache::{CacheEntry, CacheKey, Rewrite},
    pipeline::Pipeline,
    source::Source,
    walker,
};
pub(super) fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    let FileOutcome::Done {
        formatted_text: Some(text),
        ..
    } = &outcome
    else {
        return outcome;
    };
    if let Err(e) = fs_err::write(path, text) {
        return config_error(e);
    }
    outcome
}

/// Records what a cached entry knows about the rewrite. A mode that
/// skipped `run` stores `Skipped`, whereas one that ran it records
/// whether the text changed.
pub(super) fn cache_rewrite(needs_rewrite: bool, formatted_text: Option<&str>) -> Rewrite {
    if !needs_rewrite {
        return Rewrite::Skipped;
    }
    match formatted_text {
        Some(text) => Rewrite::Changed(text.to_owned()),
        None => Rewrite::Unchanged,
    }
}

pub(super) fn process_path(path: &Path, setup: &RunSetup, pass: Pass) -> FileOutcome {
    let bytes = match fs_err::read(path) {
        Ok(b) => b,
        Err(e) => return config_error(e),
    };
    // Plain `format` would persist only `run`'s post-edit diagnostics, and
    // a `--validate` check must re-confirm the rewrite parses rather than
    // trust an entry an earlier unvalidated run wrote, so both bypass the
    // cache. Every entry that remains carries `diagnose`'s as-written
    // diagnostics, so a `check` hit never replays a `run`'s.
    let needs_rewrite = matches!(pass, Pass::Both);
    let keyed = setup
        .cache
        .as_ref()
        .filter(|_| !matches!(pass, Pass::Rewrite | Pass::Diagnose { validate: true }))
        .map(|c| (c, CacheKey::compute(&bytes, &setup.config_toml)));
    if let Some(outcome) = keyed
        .as_ref()
        .and_then(|(c, k)| c.lookup(k))
        .and_then(|entry| rehydrate(path, &bytes, entry, needs_rewrite))
    {
        return outcome;
    }
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {} is not valid UTF-8: {e}", path.display());
            return FileOutcome::Failed(ExitStatus::ConfigError);
        }
    };
    let source = match Source::build(text, path.display().to_string()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: parse error in `{}`: {e}", path.display());
            return FileOutcome::Failed(ExitStatus::ParseError);
        }
    };
    let outcome = run_pipeline(source, &setup.pipeline, pass);
    if let (
        Some((c, k)),
        FileOutcome::Done {
            diagnostics,
            formatted_text,
            ..
        },
    ) = (&keyed, &outcome)
    {
        c.insert(
            k,
            &CacheEntry {
                diagnostics: diagnostics.clone(),
                rewrite: cache_rewrite(needs_rewrite, formatted_text.as_deref()),
            },
        );
    }
    outcome
}

pub(super) fn process_paths<F>(paths: &[PathBuf], handle: F) -> Vec<FileOutcome>
where
    F: Fn(&Path) -> FileOutcome + Send + Sync,
{
    walker::walk(paths)
        .par_bridge()
        .map(|entry| entry.map_or_else(walk_error, |path| handle(&path)))
        .collect()
}

pub(super) fn process_stdin<R: Read>(stdin: R, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let Ok(text) =
        io::read_to_string(stdin).inspect_err(|e| eprintln!("error: reading stdin: {e}"))
    else {
        return FileOutcome::Failed(ExitStatus::ConfigError);
    };
    text.parse::<Source>()
        .inspect_err(|e| eprintln!("error: parse error in stdin: {e}"))
        .map_or_else(
            |_| FileOutcome::Failed(ExitStatus::ParseError),
            |source| run_pipeline(source, pipeline, pass),
        )
}

pub(super) fn rehydrate(
    path: &Path,
    original_bytes: &[u8],
    entry: CacheEntry,
    needs_rewrite: bool,
) -> Option<FileOutcome> {
    let formatted_text = if needs_rewrite {
        match entry.rewrite {
            Rewrite::Changed(text) => Some(text),
            // A `check` entry skipped the rewrite this mode needs.
            Rewrite::Skipped => return None,
            Rewrite::Unchanged => None,
        }
    } else {
        None
    };
    let original_text = std::str::from_utf8(original_bytes).ok()?.to_owned();
    let file = SourceFileBuilder::new(path.display().to_string(), original_text).finish();
    Some(FileOutcome::Done {
        cached: true,
        diagnostics: entry.diagnostics,
        file,
        formatted_text,
    })
}

/// Computes only the passes `pass` reads. `check` collects the
/// as-written diagnostics, and with `--validate` set it also guards the
/// would-be rewrite against an unparseable rule output without rebuilding
/// diagnostics. A `format` run rewrites through `run`, pairing the
/// rewrite with `diagnose`'s as-written diagnostics when an output format
/// will render them, or `run`'s own otherwise.
pub(super) fn run_pipeline(source: Source, pipeline: &Pipeline, pass: Pass) -> FileOutcome {
    let file = source.source_file().clone();
    if let Pass::Diagnose { validate } = pass {
        let diagnostics = pipeline.diagnose(&source);
        if validate
            && has_format_change(&diagnostics)
            && let Err(e) = pipeline.validate(source)
        {
            return config_error(e);
        }
        return FileOutcome::Done {
            cached: false,
            diagnostics,
            file,
            formatted_text: None,
        };
    }
    let diagnosed = matches!(pass, Pass::Both).then(|| pipeline.diagnose(&source));
    match pipeline.run(source) {
        Ok((formatted, run_diagnostics)) => {
            let formatted_text = formatted
                .changed_from(file.source_text())
                .map(str::to_owned);
            FileOutcome::Done {
                cached: false,
                diagnostics: diagnosed.unwrap_or(run_diagnostics),
                file,
                formatted_text,
            }
        }
        Err(e) => config_error(e),
    }
}

pub(super) fn walk_error<E: std::fmt::Display>(err: E) -> FileOutcome {
    eprintln!("error: cannot walk: {err}");
    FileOutcome::Failed(ExitStatus::ConfigError)
}

fn config_error(e: impl std::fmt::Display) -> FileOutcome {
    eprintln!("error: {e}");
    FileOutcome::Failed(ExitStatus::ConfigError)
}
