//! Rendering one width's findings, the breaks the baseline does not carry
//! tallied by the frame and rules they share, each with the hunk around the
//! row it names and the command reproducing one of its modules alone.

use std::{
    collections::BTreeSet,
    fmt::{Display, Write},
    path::Path,
};

use crate::{
    common::{Hit, SHOWN, Tally, WIDTHS_VAR, setting},
    execute::TIMEOUT_VAR,
    outcome::Kind,
    records::{Break, Frame, Width},
    sweep::{DEFAULT_LABEL, PYTHON_VAR},
};

/// Renders one width's findings, `carried` naming the broken modules the
/// baseline already holds, which the tallies leave out so what a run shows
/// is what it newly broke.
pub(crate) fn render(carried: &BTreeSet<String>, found: &Width) -> String {
    let (raising, timeouts) = tallied(carried, found);
    let uncomparable = if found.unmeasured.is_empty() {
        found.uncomparable().to_string()
    } else {
        "unmeasured".to_owned()
    };
    let row = |label: &str, value: &dyn Display| format!("  {label:<12} {value:>5}");
    let mut lines = vec![
        row("candidates", &found.candidates),
        row("comparable", &found.comparable),
        row("uncomparable", &uncomparable),
        row("breaks", &found.breaks.len()),
        row("timeouts", &found.timing_out()),
        row("flaky", &found.flaky.len()),
    ];
    if !carried.is_empty() {
        lines.push(row("carried", &carried.len()));
    }
    if found.refused > 0 {
        lines.push(row("refused", &found.refused));
    }
    let mut rendered = lines.join("\n");
    rendered.push_str(&raising.render("raises or rebinds"));
    rendered.push_str(&timeouts.render("times out"));
    for (heading, listed) in [
        ("flaky, a second run did not confirm it", &found.flaky),
        ("unmeasured, a run left no record", &found.unmeasured),
    ] {
        if !listed.is_empty() {
            let _ = write!(rendered, "\n\n{heading} ({}):", listed.len());
            for module in listed.iter().take(SHOWN) {
                let _ = write!(rendered, "\n  {module}");
            }
            let rest = listed.len().saturating_sub(SHOWN);
            if rest > 0 {
                let _ = write!(rendered, "\n  ... and {rest} more");
            }
        }
    }
    rendered
}

/// The sentence naming where a break raises, why, and what it traces to,
/// which is the wording a tally keys it by.
fn defect(brk: &Break) -> String {
    let Frame { file, row } = &brk.frame;
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
    knobs.push("mise run imports".to_owned());
    format!("{} {module}", knobs.join(" "))
}

/// The breaks the baseline does not carry, split into the ones that raised
/// or rebound and the ones that outran their deadline, each keyed by the
/// sentence they share so one frame reaching many modules reports once.
fn tallied(carried: &BTreeSet<String>, found: &Width) -> (Tally, Tally) {
    let mut raising = Tally::default();
    let mut timeouts = Tally::default();
    for brk in found
        .breaks
        .iter()
        .filter(|brk| !carried.contains(&brk.module))
    {
        let tally = if brk.formatted.kind == Kind::Timeout {
            &mut timeouts
        } else {
            &mut raising
        };
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
    (raising, timeouts)
}
