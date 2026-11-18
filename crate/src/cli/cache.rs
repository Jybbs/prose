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
    let now = SystemTime::now();
    if let Some(t) = info.oldest_mtime {
        writeln!(stdout, "oldest: {}", relative_age(t, now)).context("writing stdout")?;
    }
    if let Some(t) = info.newest_mtime {
        writeln!(stdout, "newest: {}", relative_age(t, now)).context("writing stdout")?;
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

/// Renders how long before `now` the instant `t` falls, in its largest
/// whole unit.
fn relative_age(t: SystemTime, now: SystemTime) -> String {
    let Ok(d) = now.duration_since(t) else {
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
    use std::time::Duration;

    use rstest::rstest;

    use super::*;

    const NOW: SystemTime = SystemTime::UNIX_EPOCH;

    #[test]
    fn relative_age_renders_future_when_mtime_lies_ahead_of_now() {
        assert_eq!(
            relative_age(NOW + Duration::from_secs(1), NOW),
            "in the future"
        );
    }

    #[rstest]
    #[case::now(0, "0s ago")]
    #[case::last_second(59, "59s ago")]
    #[case::first_minute(60, "1m ago")]
    #[case::last_minute(3_599, "59m ago")]
    #[case::first_hour(3_600, "1h ago")]
    #[case::last_hour(86_399, "23h ago")]
    #[case::first_day(86_400, "1d ago")]
    #[case::two_days(172_800, "2d ago")]
    fn relative_age_renders_the_largest_whole_unit(#[case] seconds: u64, #[case] rendered: &str) {
        assert_eq!(
            relative_age(NOW - Duration::from_secs(seconds), NOW),
            rendered
        );
    }
}
