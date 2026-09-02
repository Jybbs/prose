//! `prose cache` subcommand handlers and their shared helpers.

use std::{
    io::{self, Write},
    time::SystemTime,
};

use anyhow::Context;

use super::{exit_status::ExitStatus, load_config_or_status};
use crate::{
    cache::{Cache, CleanReport},
    config::NoticeDedup,
};

pub(crate) fn clean<W: Write>(stdout: W) -> anyhow::Result<ExitStatus> {
    let report = match or_status(Cache::open().and_then(|c| c.clean())) {
        Ok(report) => report,
        Err(status) => return Ok(status),
    };
    write_report(stdout, report)?;
    Ok(ExitStatus::Clean)
}

pub(crate) fn compact<W: Write>(stdout: W) -> anyhow::Result<ExitStatus> {
    let config = match load_config_or_status(&NoticeDedup::default()) {
        Ok(c) => c,
        Err(s) => return Ok(s),
    };
    let cache = match or_status(Cache::open()) {
        Ok(c) => c
            .with_max_entries(config.cache.max_entries)
            .with_max_size_mib(config.cache.max_size_mib),
        Err(s) => return Ok(s),
    };
    write_report(stdout, cache.compact())?;
    Ok(ExitStatus::Clean)
}

pub(crate) fn info<W: Write>(mut stdout: W) -> anyhow::Result<ExitStatus> {
    let cache = match or_status(Cache::open()) {
        Ok(c) => c,
        Err(s) => return Ok(s),
    };
    let info = cache.info();
    writeln!(
        stdout,
        "path: {}\nentries: {}\nbytes: {}",
        info.path.display(),
        info.entries,
        info.bytes
    )
    .context("writing stdout")?;
    if let Some(t) = info.oldest_mtime {
        writeln!(stdout, "oldest: {}", relative_age(t)).context("writing stdout")?;
    }
    if let Some(t) = info.newest_mtime {
        writeln!(stdout, "newest: {}", relative_age(t)).context("writing stdout")?;
    }
    Ok(ExitStatus::Clean)
}

/// `result`'s value, or the status a failed cache operation exits with
/// once its error is reported.
fn or_status<T>(result: io::Result<T>) -> Result<T, ExitStatus> {
    result.map_err(|e| {
        eprintln!("error: {e}");
        ExitStatus::ConfigError
    })
}

fn relative_age(t: SystemTime) -> String {
    let Ok(d) = SystemTime::now().duration_since(t) else {
        return "in the future".to_owned();
    };
    let (n, unit) = match d.as_secs() {
        s @ 0..60 => (s, "s"),
        s @ 60..3600 => (s / 60, "m"),
        s @ 3600..86400 => (s / 3600, "h"),
        s => (s / 86400, "d"),
    };
    format!("{n}{unit} ago")
}

fn write_report<W: Write>(mut stdout: W, report: CleanReport) -> anyhow::Result<()> {
    writeln!(
        stdout,
        "removed {} entries ({} bytes)",
        report.entries, report.bytes
    )
    .context("writing stdout")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_age_renders_future_when_mtime_lies_ahead_of_now() {
        let future = SystemTime::now() + std::time::Duration::from_mins(1);
        assert_eq!(relative_age(future), "in the future");
    }

    #[test]
    fn relative_age_renders_seconds_minutes_hours_days() {
        let now = SystemTime::now();
        assert!(relative_age(now - std::time::Duration::from_secs(5)).ends_with("s ago"));
        assert!(relative_age(now - std::time::Duration::from_mins(2)).ends_with("m ago"));
        assert!(relative_age(now - std::time::Duration::from_hours(2)).ends_with("h ago"));
        assert!(relative_age(now - std::time::Duration::from_hours(48)).ends_with("d ago"));
    }
}
