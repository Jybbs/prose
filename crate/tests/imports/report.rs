//! Render one width's findings, the breaks the baseline does not carry
//! tallied by the frame and rules they share, each with the hunk around the
//! row it names and the command reproducing one of its modules alone.

use std::{collections::BTreeSet, path::Path};

use crate::{
    common::{Hit, Tally, WIDTHS_VAR, setting},
    records::{Break, Kind, Width},
    sweep::{DEFAULT_LABEL, MODULE_VAR, PYTHON_VAR, TIMEOUT_VAR},
};

/// Renders one width's findings, `carried` naming the broken modules the
/// baseline already holds, which the tallies leave out so what a run shows
/// is what it newly broke.
pub(crate) fn render(carried: &BTreeSet<String>, found: &Width) -> String {
    let timeouts = tallied(carried, found, Kind::Timeout);
    let raising = tallied(carried, found, Kind::Raised);
    let uncomparable = if found.unmeasured.is_empty() {
        found.uncomparable().to_string()
    } else {
        "unmeasured".to_owned()
    };
    let row = |label: &str, value: &str| format!("  {label:<12} {value:>5}");
    let mut lines = vec![
        row("candidates", &found.candidates.to_string()),
        row("comparable", &found.comparable.to_string()),
        row("uncomparable", &uncomparable),
        row("breaks", &found.breaks.len().to_string()),
        row("timeouts", &timeouts.len().to_string()),
        row("flaky", &found.flaky.len().to_string()),
    ];
    if !carried.is_empty() {
        lines.push(row("carried", &carried.len().to_string()));
    }
    if found.refused > 0 {
        lines.push(row("refused", &found.refused.to_string()));
    }
    let mut rendered = lines.join("\n");
    rendered.push_str(&raising.render("raises or rebinds"));
    rendered.push_str(&timeouts.render("times out"));
    for (heading, listed) in [
        ("flaky, a second run did not confirm it", &found.flaky),
        ("unmeasured, a run left no record", &found.unmeasured),
    ] {
        if !listed.is_empty() {
            rendered.push_str(&format!("\n\n{heading} ({}):", listed.len()));
            for module in listed {
                rendered.push_str(&format!("\n  {module}"));
            }
        }
    }
    rendered
}

/// The sentence naming where a break raises, why, and what it traces to,
/// which is the wording a tally keys it by.
fn defect(brk: &Break) -> String {
    let (file, row) = &brk.frame;
    let at = row.map_or_else(|| file.clone(), |row| format!("{file}:{row}"));
    format!("{at} {}, {}", brk.reason, brk.attribution)
}

/// The command that runs one module on its own, carrying every knob the
/// current run set.
fn reproduction(label: &str, module: &str) -> String {
    let mut knobs: Vec<_> = [PYTHON_VAR, TIMEOUT_VAR]
        .iter()
        .filter_map(|knob| Some(format!("{knob}={}", setting(knob)?)))
        .collect();
    if label != DEFAULT_LABEL {
        knobs.push(format!("{WIDTHS_VAR}={label}"));
    }
    knobs.push(format!("{MODULE_VAR}={module}"));
    format!("{} mise run imports", knobs.join(" "))
}

/// The breaks of one kind the baseline does not carry, keyed by the sentence
/// they share so one frame reaching many modules reports once.
fn tallied(carried: &BTreeSet<String>, found: &Width, kind: Kind) -> Tally {
    let mut tally = Tally::default();
    for brk in found.breaks.iter().filter(|brk| {
        !carried.contains(&brk.module)
            && (brk.formatted.kind == Kind::Timeout) == (kind == Kind::Timeout)
    }) {
        tally.record_hit(
            defect(brk),
            Path::new(&brk.module),
            Hit {
                clause: None,
                detail: Some(brk.hunk.join("\n")),
                repro: Some(reproduction(&found.label, &brk.module)),
            },
        );
    }
    tally
}
