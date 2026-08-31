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

/// One tagged cycle's summary record and the files each rule fired on.
struct Cycle {
    counts: BTreeMap<String, usize>,
    fired: FxHashMap<String, BTreeSet<String>>,
    unstable: usize,
}

impl Cycle {
    /// Reads one cycle's records, the summary carrying the per-rule counts
    /// and the rest naming a rule beside the file its fix landed in.
    fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut cycle = Self {
            counts: BTreeMap::new(),
            fired: FxHashMap::default(),
            unstable: 0,
        };
        for line in fs_err::read_to_string(path)?.lines() {
            let record: Value = serde_json::from_str(line)?;
            if record.get("kind").and_then(Value::as_str) == Some("summary") {
                cycle.counts = serde_json::from_value(record["rules_fired"].clone())?;
                cycle.unstable = record["unstable"].as_array().map_or(0, Vec::len);
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

/// The two cycles one width compares, rendered as terminal text or as the
/// Markdown a step summary takes.
struct Report {
    base: Cycle,
    head: Cycle,
    markdown: bool,
    stage: PathBuf,
    width: String,
}

impl Report {
    /// Reads both sides of one width out of the stage.
    fn read(stage: &Path, width: &str, markdown: bool) -> Result<Self, Box<dyn Error>> {
        let side = |name: &str| stage.join(".git").join(format!("{name}-{width}.ndjson"));
        Ok(Self {
            base: Cycle::read(&side("base"))?,
            head: Cycle::read(&side("head"))?,
            markdown,
            stage: stage.to_owned(),
            width: width.to_owned(),
        })
    }

    /// Renders `text` as inline code in Markdown, bare otherwise.
    fn code(&self, text: &str) -> String {
        if self.markdown {
            format!("`{text}`")
        } else {
            text.to_owned()
        }
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
            return self.lines("every rule fired the same number of times");
        }
        let mut table = Builder::default();
        table.push_record(["Rule", "Base", "Head", "Delta"]);
        for (slug, before, after, delta) in moved {
            table.push_record([
                self.code(slug),
                before.to_string(),
                after.to_string(),
                format!("{delta:+}"),
            ]);
        }
        let mut built = table.build();
        if self.markdown {
            return format!("{}\n\n", built.with(Style::markdown()));
        }
        self.lines(&built.with(Style::blank()).to_string())
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
        if self.markdown {
            return Ok(format!("```\n{shown}\n```\n\n"));
        }
        Ok(self.lines(shown))
    }

    /// Renders `text` as a bullet list in Markdown, indented lines otherwise.
    fn lines(&self, text: &str) -> String {
        let marker = if self.markdown { "- " } else { "  " };
        let body: String = text
            .lines()
            .map(|line| format!("{marker}{line}\n"))
            .collect();
        if self.markdown {
            format!("{body}\n")
        } else {
            body
        }
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
            return self.lines("every rule fires on the same files");
        }
        let rendered = moves
            .iter()
            .map(|(slug, verb, files)| {
                let plural = if files.len() > 1 { "s" } else { "" };
                format!(
                    "{} {verb} on {} file{plural} ({})",
                    self.code(slug),
                    files.len(),
                    self.named(files)
                )
            })
            .join("\n");
        self.lines(&rendered)
    }

    /// Lists the first [`SHOWN`] of `files` and counts the rest.
    fn named(&self, files: &[String]) -> String {
        let names = files
            .iter()
            .take(SHOWN)
            .map(|name| self.code(name))
            .join(", ");
        match files.len().saturating_sub(SHOWN) {
            0 => names,
            rest => format!("{names}, and {rest} more"),
        }
    }

    /// Renders the width's heading, count table, movements, stability, and
    /// diffstat.
    fn render(&self) -> Result<String, Box<dyn Error>> {
        let heading = if self.markdown {
            format!("### Width {}\n\n", self.width)
        } else {
            format!("width {}\n", self.width)
        };
        Ok(heading + &self.counts() + &self.movements() + &self.stability() + &self.diffstat()?)
    }

    /// Names a side whose summary carries unstable entries at this width.
    fn stability(&self) -> String {
        let notes = [("base", &self.base), ("head", &self.head)]
            .into_iter()
            .filter(|(_, cycle)| cycle.unstable > 0)
            .map(|(side, cycle)| {
                format!(
                    "{side} is unstable on {} of the files at this width",
                    cycle.unstable
                )
            })
            .join("\n");
        if notes.is_empty() {
            String::new()
        } else {
            self.lines(&notes)
        }
    }
}

/// Renders the delta between a stage's base and head cycles.
#[derive(Parser)]
struct Args {
    /// Render the shape a step summary takes.
    #[arg(long)]
    markdown: bool,
    /// The stage holding both cycles' tags and records.
    stage: PathBuf,
    /// The widths to report, one section apiece.
    #[arg(required = true)]
    widths: Vec<String>,
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

/// Names the baseline the stage was baked from and the head it was read
/// against.
fn heading(stage: &Path) -> Result<String, Box<dyn Error>> {
    let baseline = fs_err::read_to_string(stage.join(".git").join("baseline"))?;
    let head = git(&["describe", "--always", "--dirty"])?;
    Ok(format!(
        "## 🦋 Delta\n\nBaseline `{}` against head `{}`.\n\n",
        baseline.trim(),
        head.trim()
    ))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let mut text = String::new();
    for width in &args.widths {
        text += &Report::read(&args.stage, width, args.markdown)?.render()?;
    }
    if args.markdown {
        text = heading(&args.stage)? + &text;
    }
    match std::env::var("GITHUB_STEP_SUMMARY") {
        Ok(path) if args.markdown => fs_err::write(path, text)?,
        _ => print!("{text}"),
    }
    Ok(())
}
