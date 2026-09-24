//! Rendering one width's findings. The breaks are tallied by the frame and
//! rules they share, each shown with the hunk around the row it names and the
//! command that reproduces one of its modules alone. Beneath them the report
//! lists each file the pipeline could not format, each module whose original
//! run did not end cleanly, and each name the comparison left out.

use std::{
    collections::BTreeMap,
    fmt::{Display, Write},
    path::Path,
};

use itertools::Itertools;

use crate::{
    common::{Hit, SHOWN, Tally, WIDTHS_VAR, remainder, setting},
    corpus::PYTHON_VAR,
    execute::TIMEOUT_VAR,
    outcome::Kind,
    records::{Break, Frame, Width},
    sweep::DEFAULT_LABEL,
};

/// Renders one width's findings.
pub(crate) fn render(found: &Width) -> String {
    let (raising, rebinding, timeouts) = tallied(found);
    let uncomparable = if found.unmeasured.is_empty() {
        found.uncomparable.len().to_string()
    } else {
        "unmeasured".to_owned()
    };
    let row = |label: &str, value: &dyn Display| format!("  {label:<12} {value:>5}");
    let mut lines = vec![
        row("candidates", &found.candidates),
        row("rewritten", &found.rewritten),
        row("comparable", &found.comparable),
        row("uncomparable", &uncomparable),
        row("breaks", &found.broken()),
        row("raises", &found.counting(Kind::Raised)),
        row("rebinds", &found.counting(Kind::Ok)),
        row("timeouts", &found.counting(Kind::Timeout)),
        row("rejected", &found.rejected.len()),
        row("flaky", &found.flaky.len()),
        row("varying", &found.varying()),
        row("left out", &found.left_out()),
    ];
    if found.unread > 0 {
        lines.push(row("unread", &found.unread));
    }
    let mut rendered = lines.join("\n");
    if !found.uncomparable.is_empty() {
        let split = found
            .reaches()
            .iter()
            .map(|(reach, counted)| format!("{counted} {reach}"))
            .join(", ");
        let _ = write!(
            rendered,
            "\n\nuncomparable by reach ({}):\n  {split}",
            found.uncomparable.len(),
        );
    }
    rendered.push_str(&raising.render("raises"));
    rendered.push_str(&rebinding.render("runs and binds a different namespace"));
    rendered.push_str(&timeouts.render("times out"));
    let rejected: Vec<_> = found
        .rejected
        .iter()
        .map(|(file, error)| format!("{file}  {error}"))
        .collect();
    let blocked: Vec<_> = found
        .uncomparable
        .iter()
        .map(|(module, left)| format!("{module}  {}, {}", left.reach, left.reason))
        .collect();
    let left_out: Vec<_> = found
        .removed
        .values()
        .flat_map(BTreeMap::values)
        .cloned()
        .collect();
    let varied = excluded(found);
    for (heading, listed) in [
        ("rejected, the pipeline could not format it", &rejected),
        ("uncomparable, the original did not run cleanly", &blocked),
        ("left out, a recorded fix removed the binding", &left_out),
        ("flaky, a second run varied", &varied),
        ("unmeasured, a run left no record", &found.unmeasured),
    ] {
        if !listed.is_empty() {
            let _ = write!(
                rendered,
                "\n\n{heading} ({}):\n  {}{}",
                listed.len(),
                listed.iter().take(SHOWN).format("\n  "),
                remainder(listed.len()),
            );
        }
    }
    rendered
}

/// The sentence naming where a break raises, why it raises, and what it
/// traces to, which is the wording a tally keys it under.
fn defect(brk: &Break) -> String {
    let Frame { file, row } = &brk.frame;
    let at = row.map_or_else(|| file.clone(), |row| format!("{file}:{row}"));
    format!("{at} {}, {}", brk.reason, brk.attribution)
}

/// The rows naming each module a run set aside, one per module beside the
/// names it varied on. A module whose whole namespace varied carries no
/// names, so its row reads `the whole namespace` in their place.
fn excluded(found: &Width) -> Vec<String> {
    found
        .flaky
        .iter()
        .map(|(module, names)| {
            if names.is_empty() {
                format!("{module}  the whole namespace")
            } else {
                format!("{module}  {}", names.iter().format(", "))
            }
        })
        .collect()
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

/// The breaks, split by how the formatted run ended, each keyed by the
/// sentence they share so one frame reaching many modules reports once.
fn tallied(found: &Width) -> (Tally, Tally, Tally) {
    let mut raising = Tally::default();
    let mut rebinding = Tally::default();
    let mut timeouts = Tally::default();
    for brk in &found.breaks {
        let tally = match brk.formatted.kind {
            Kind::Ok => &mut rebinding,
            Kind::Raised => &mut raising,
            Kind::Timeout => &mut timeouts,
            Kind::Unmeasured => {
                unreachable!(
                    "invariant: a run that left no record is unmeasured rather than broken"
                )
            }
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
    (raising, rebinding, timeouts)
}
