//! Renders the delta between a stage's base and head cycles at each width.
//!
//! Reads `<stage>/.git/base-<width>.ndjson` and `head-<width>.ndjson`, each
//! carrying a run's summary record and one `{code, filename}` record per fix,
//! and renders per width the rules whose firing count moved, sorted by the
//! size of the move, the files each rule newly fires on or no longer fires
//! on, and git's diffstat between the two tags.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde_json::Deserializer;
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
#[derive(Default)]
struct Cycle {
    counts: BTreeMap<String, usize>,
    fired: FxHashMap<String, BTreeSet<String>>,
}

impl Cycle {
    /// Reads one cycle's records out of the file at `path`, folding each into
    /// the counts or into the per-rule file set.
    fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut cycle = Self::default();
        for record in Deserializer::from_str(&fs_err::read_to_string(path)?).into_iter::<Record>() {
            match record? {
                Record::Fix { code, filename } => {
                    cycle.fired.entry(code).or_default().insert(filename);
                }
                Record::Summary { rules_fired } => cycle.counts = rules_fired,
            }
        }
        Ok(cycle)
    }
}

/// One line of a cycle's stream, where a fix line names its rule and the file
/// the fix landed in, and a summary line carries the per-rule counts.
#[derive(Deserialize)]
#[serde(untagged)]
enum Record {
    Fix {
        code: String,
        filename: String,
    },
    Summary {
        rules_fired: BTreeMap<String, usize>,
    },
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

    /// Renders as a table the rules whose firing count moved, largest move first.
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
                let delta = after.cast_signed() - before.cast_signed();
                (delta != 0).then_some((slug, before, after, delta))
            })
            .sorted_by_key(|&(slug, _, _, delta)| (Reverse(delta.abs()), slug))
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

    /// Returns git's own diffstat between the two tags, capped at five files.
    fn diffstat(&self) -> Result<String, Box<dyn Error>> {
        let text = git(
            &self.stage,
            &[
                "diff",
                "--stat-count=5",
                &format!("base-{}", self.width),
                &format!("head-{}", self.width),
            ],
        )?;
        let trimmed = text.lines().map(str::trim).join("\n");
        Ok(section(&trimmed, "no file differs"))
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
            .sorted_by_key(|(slug, verb, files)| (Reverse(files.len()), *slug, *verb))
            .collect_vec();
        let rendered = moves
            .iter()
            .map(|(slug, verb, files)| {
                let plural = if files.len() > 1 { "s" } else { "" };
                let count = files.len();
                let names = Self::named(files);
                format!("{slug} {verb} on {count} file{plural} ({names})")
            })
            .join("\n");
        section(&rendered, "every rule fires on the same files")
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
        let diffstat = self.diffstat()?;
        Ok(format!(
            "width {}\n{}{}{diffstat}",
            self.width,
            self.counts(),
            self.movements()
        ))
    }
}

/// Runs `git` with `args` inside `dir`, failing on a nonzero exit.
fn git(dir: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let run = Command::new("git").current_dir(dir).args(args).output()?;
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
    text.lines().flat_map(|line| ["  ", line, "\n"]).collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    for width in &args.widths {
        print!("{}", Report::read(&args.stage, width)?.render()?);
    }
    Ok(())
}

/// Renders `text` indented, or `empty` where there is nothing to render.
fn section(text: &str, empty: &str) -> String {
    indented(if text.is_empty() { empty } else { text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_counts_the_files_past_the_cap() {
        let files: Vec<String> = (1..=5).map(|n| format!("f{n}.py")).collect();
        assert_eq!(Report::named(&files), "f1.py, f2.py, f3.py, and 2 more");
    }

    #[test]
    fn named_lists_every_file_at_the_cap() {
        let files: Vec<String> = (1..=3).map(|n| format!("f{n}.py")).collect();
        assert_eq!(Report::named(&files), "f1.py, f2.py, f3.py");
    }

    #[test]
    fn section_falls_back_where_there_is_nothing() {
        assert_eq!(section("", "nothing"), "  nothing\n");
    }

    #[test]
    fn section_indents_its_text() {
        assert_eq!(section("one\ntwo", "nothing"), "  one\n  two\n");
    }
}
