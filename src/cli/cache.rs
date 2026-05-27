//! `prose cache` subcommand handlers and their shared helpers.

use std::io::Write;
use std::time::SystemTime;

use anyhow::Context;

use super::exit_status::ExitStatus;
use super::log_error_chain;
use crate::cache::Cache;
use crate::config::Config;

pub(crate) fn clean<W: Write>(mut stdout: W) -> anyhow::Result<ExitStatus> {
    match Cache::open().and_then(|c| c.clean()) {
        Ok(report) => {
            writeln!(
                stdout,
                "removed {entries} entries ({bytes} bytes)",
                entries = report.entries,
                bytes = report.bytes,
            )
            .context("writing stdout")?;
            Ok(ExitStatus::Clean)
        }
        Err(err) => {
            eprintln!("error: {err}");
            Ok(ExitStatus::ConfigError)
        }
    }
}

pub(crate) fn compact<W: Write>(mut stdout: W) -> anyhow::Result<ExitStatus> {
    let cwd = std::env::current_dir().context("reading current working directory")?;
    let config = match Config::load(&cwd).context("loading [tool.prose] config") {
        Ok(c) => c,
        Err(e) => {
            log_error_chain(&e);
            return Ok(ExitStatus::ConfigError);
        }
    };
    let cache = match Cache::open() {
        Ok(c) => c.with_max_size_mib(config.cache.max_size_mib),
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(ExitStatus::ConfigError);
        }
    };
    let report = cache.compact();
    writeln!(
        stdout,
        "removed {entries} entries ({bytes} bytes)",
        entries = report.entries,
        bytes = report.bytes,
    )
    .context("writing stdout")?;
    Ok(ExitStatus::Clean)
}

pub(crate) fn info<W: Write>(mut stdout: W) -> anyhow::Result<ExitStatus> {
    let cache = match Cache::open() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(ExitStatus::ConfigError);
        }
    };
    let info = cache.info();
    writeln!(stdout, "path: {}", info.path.display()).context("writing stdout")?;
    writeln!(stdout, "entries: {}", info.entries).context("writing stdout")?;
    writeln!(stdout, "bytes: {}", info.bytes).context("writing stdout")?;
    if let Some(t) = info.oldest_mtime {
        writeln!(stdout, "oldest: {}", relative_age(t)).context("writing stdout")?;
    }
    if let Some(t) = info.newest_mtime {
        writeln!(stdout, "newest: {}", relative_age(t)).context("writing stdout")?;
    }
    Ok(ExitStatus::Clean)
}

fn relative_age(t: SystemTime) -> String {
    let Ok(d) = SystemTime::now().duration_since(t) else {
        return "in the future".to_owned();
    };
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_age_renders_seconds_minutes_hours_days() {
        let now = SystemTime::now();
        assert!(relative_age(now - std::time::Duration::from_secs(5)).ends_with("s ago"));
        assert!(relative_age(now - std::time::Duration::from_secs(120)).ends_with("m ago"));
        assert!(relative_age(now - std::time::Duration::from_secs(7200)).ends_with("h ago"));
        assert!(relative_age(now - std::time::Duration::from_secs(172_800)).ends_with("d ago"));
    }
}
