//! Renders the delta between a stage's base and head cycles at each width.
//!
//! Reads `<stage>/.git/base-<width>.ndjson` and `head-<width>.ndjson`, each
//! carrying a run's summary record and one `{code, filename}` record per fix,
//! and renders per width the rules whose firing count moved, sorted by the
//! size of the move, the files each rule newly fires on or no longer fires
//! on, and git's diffstat between the two tags.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tabled::{builder::Builder, settings::Style};

/// How many files a movement line names before it counts the rest.
const SHOWN: usize = 3;

/// Renders the delta between a stage's base and head cycles.
#[derive(Parser)]
struct Args {
    /// The stage holding both cycles' tags and records.
    stage: PathBuf,
    /// The widths to report, one section apiece.
    #[arg(required = true)]
    widths: Vec<String>,
}

/// One tagged cycle's summary record and the files each rule fired on.
struct Cycle {
    counts: BTreeMap<String, usize>,
    fired: FxHashMap<String, BTreeSet<String>>,
}

impl Cycle {
    /// Reads one cycle's records, the summary carrying the per-rule counts
    /// and the rest naming a rule beside the file its fix landed in.
    fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut cycle = Self {
            counts: BTreeMap::new(),
            fired: FxHashMap::default(),
        };
        for line in fs_err::read_to_string(path)?.lines() {
            let record: Value = serde_json::from_str(line)?;
            if record.get("kind").and_then(Value::as_str) == Some("summary") {
                cycle.counts = serde_json::from_value(record["rules_fired"].clone())?;
            } else if let (Some(code), Some(file)) = (
                record.get("code").and_then(Value::as_str),
                record.get("filename").and_then(Value::as_str),
            ) {
                cycle
                    .fired
                    .entry(code.to_owned())
                    .or_default()
                    .insert(file.to_owned());
            }
        }
        Ok(cycle)
    }
}

/// The two cycles one width compares, rendered as terminal text.
struct Report {
    base: Cycle,
    head: Cycle,
    stage: PathBuf,
    width: String,
}

impl Report {
    /// Reads both sides of one width out of the stage.
    fn read(stage: &Path, width: &str) -> Result<Self, Box<dyn Error>> {
        let side = |name: &str| stage.join(".git").join(format!("{name}-{width}.ndjson"));
        Ok(Self {
            base: Cycle::read(&side("base"))?,
            head: Cycle::read(&side("head"))?,
            stage: stage.to_owned(),
            width: width.to_owned(),
        })
    }

    /// Tables the rules whose firing count moved, largest move first.
    fn counts(&self) -> String {
        let moved = self
            .base
            .counts
            .keys()
            .chain(self.head.counts.keys())
            .unique()
            .filter_map(|slug| {
                let before = self.base.counts.get(slug).copied().unwrap_or(0);
                let after = self.head.counts.get(slug).copied().unwrap_or(0);
                (before != after).then(|| (slug, before, after, after as isize - before as isize))
            })
            .sorted_by(|a, b| b.3.abs().cmp(&a.3.abs()).then_with(|| a.0.cmp(b.0)))
            .collect_vec();
        if moved.is_empty() {
            return indented("every rule fired the same number of times");
        }
        let mut table = Builder::default();
        table.push_record(["Rule", "Base", "Head", "Delta"]);
        for (slug, before, after, delta) in moved {
            table.push_record([
                slug.clone(),
                before.to_string(),
                after.to_string(),
                format!("{delta:+}"),
            ]);
        }
        indented(&table.build().with(Style::blank()).to_string())
    }

    /// Git's own diffstat between the two tags, capped at five files.
    fn diffstat(&self) -> Result<String, Box<dyn Error>> {
        let stage = self.stage.display().to_string();
        let text = git(&[
            "-C",
            &stage,
            "diff",
            "--stat-count=5",
            &format!("base-{}", self.width),
            &format!("head-{}", self.width),
        ])?;
        let trimmed = text.lines().map(str::trim).join("\n");
        let shown = if trimmed.is_empty() {
            "no file differs"
        } else {
            &trimmed
        };
        Ok(indented(shown))
    }

    /// Names the files each rule newly fires on or no longer fires on.
    fn movements(&self) -> String {
        let empty = BTreeSet::new();
        let moves = self
            .base
            .fired
            .keys()
            .chain(self.head.fired.keys())
            .unique()
            .flat_map(|slug| {
                let base = self.base.fired.get(slug).unwrap_or(&empty);
                let head = self.head.fired.get(slug).unwrap_or(&empty);
                [
                    ("newly fires", head.difference(base).cloned().collect_vec()),
                    (
                        "no longer fires",
                        base.difference(head).cloned().collect_vec(),
                    ),
                ]
                .into_iter()
                .filter(|(_, files)| !files.is_empty())
                .map(move |(verb, files)| (slug, verb, files))
            })
            .sorted_by(|a, b| {
                b.2.len()
                    .cmp(&a.2.len())
                    .then_with(|| a.0.cmp(b.0))
                    .then_with(|| a.1.cmp(b.1))
            })
            .collect_vec();
        if moves.is_empty() {
            return indented("every rule fires on the same files");
        }
        let rendered = moves
            .iter()
            .map(|(slug, verb, files)| {
                let plural = if files.len() > 1 { "s" } else { "" };
                format!(
                    "{} {verb} on {} file{plural} ({})",
                    slug,
                    files.len(),
                    Self::named(files)
                )
            })
            .join("\n");
        indented(&rendered)
    }

    /// Lists the first [`SHOWN`] of `files` and counts the rest.
    fn named(files: &[String]) -> String {
        let names = files.iter().take(SHOWN).join(", ");
        match files.len().saturating_sub(SHOWN) {
            0 => names,
            rest => format!("{names}, and {rest} more"),
        }
    }

    /// Renders the width's heading, count table, movements, and diffstat.
    fn render(&self) -> Result<String, Box<dyn Error>> {
        let heading = format!("width {}\n", self.width);
        Ok(heading + &self.counts() + &self.movements() + &self.diffstat()?)
    }
}

/// Runs `git` with `args`, failing rather than reporting empty output when
/// the command does not succeed.
fn git(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let run = Command::new("git").args(args).output()?;
    if !run.status.success() {
        return Err(format!(
            "git {} exited {}: {}",
            args.join(" "),
            run.status,
            String::from_utf8_lossy(&run.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(run.stdout)?)
}

/// Renders `text` as indented lines.
fn indented(text: &str) -> String {
    text.lines().map(|line| format!("  {line}\n")).collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    for width in &args.widths {
        print!("{}", Report::read(&args.stage, width)?.render()?);
    }
    Ok(())
}
