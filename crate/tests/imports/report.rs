//! Render one width's findings, the breaks grouped by the frame and rules
//! they share, each group with the hunk around the row it names and the
//! command reproducing one of its modules alone.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
};

use itertools::Itertools;

use crate::{
    records::{Break, Kind, Width},
    sweep::{DEFAULT_LABEL, MODULE_VAR, PYTHON_VAR, TIMEOUT_VAR, WIDTHS_VAR},
};

/// How many broken modules a group names before it counts the rest.
const SHOWN: usize = 30;

/// Renders one width's findings, `carried` naming the broken modules the
/// baseline already holds.
pub(crate) fn render(carried: &BTreeSet<String>, found: &Width) -> String {
    let timeouts: Vec<_> = found
        .breaks
        .iter()
        .filter(|brk| brk.formatted.kind == Kind::Timeout)
        .collect();
    let raising: Vec<_> = found
        .breaks
        .iter()
        .filter(|brk| brk.formatted.kind != Kind::Timeout)
        .collect();
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
    for (heading, listed) in [("raises or rebinds", raising), ("times out", timeouts)] {
        if listed.is_empty() {
            continue;
        }
        let mut seats: BTreeMap<(&(String, Option<usize>), &String), usize> = BTreeMap::new();
        let mut groups: Vec<Vec<&Break>> = Vec::new();
        for brk in &listed {
            let key = (&brk.frame, &brk.attribution);
            let seat = *seats.entry(key).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[seat].push(brk);
        }
        lines.push(String::new());
        lines.push(format!(
            "  {heading} ({} modules at {} frames):",
            listed.len(),
            groups.len()
        ));
        for members in &groups {
            lines.extend(rendered_group(carried, &found.label, members));
        }
    }
    for (heading, listed) in [
        ("flaky, a second run did not confirm it", &found.flaky),
        ("unmeasured, a run left no record", &found.unmeasured),
    ] {
        if !listed.is_empty() {
            lines.push(String::new());
            lines.push(format!("  {heading} ({}):", listed.len()));
            lines.extend(listed.iter().map(|module| format!("    {module}")));
        }
    }
    lines.join("\n")
}

/// Renders the breaks that share a frame and an attribution, the hunk once,
/// a reason every member shares once, the modules the baseline does not carry
/// ahead of the ones it does, and up to [`SHOWN`] modules with the rest
/// counted.
fn rendered_group(carried: &BTreeSet<String>, label: &str, members: &[&Break]) -> Vec<String> {
    let leader = members[0];
    let (file, row) = &leader.frame;
    let shared = (members.iter().map(|brk| &brk.reason).unique().count() == 1)
        .then(|| leader.reason.clone());
    let ordered: Vec<_> = members
        .iter()
        .sorted_by(|a, b| {
            (carried.contains(&a.module), &a.module).cmp(&(carried.contains(&b.module), &b.module))
        })
        .collect();
    let at = row.map_or_else(|| file.clone(), |row| format!("{file}:{row}"));
    let mut lines = vec![format!("    {at} {}", leader.attribution)];
    if let Some(reason) = &shared {
        lines.push(format!("      each {reason}"));
    }
    lines.extend(leader.hunk.iter().map(|line| format!("      {line}")));
    lines.extend(ordered.iter().take(SHOWN).map(|brk| {
        let reason = shared
            .as_ref()
            .map_or_else(|| format!(" {}", brk.reason), |_| String::new());
        let held = if carried.contains(&brk.module) {
            ", carried by the baseline"
        } else {
            ""
        };
        format!("      {}{reason}{held}", brk.module)
    }));
    if ordered.len() > SHOWN {
        lines.push(format!("      ... and {} more", ordered.len() - SHOWN));
    }
    lines.push(format!(
        "      reproduce with {}",
        reproduction(label, &ordered[0].module)
    ));
    lines
}

/// The command that runs one module on its own, carrying every knob the
/// current run set.
fn reproduction(label: &str, module: &str) -> String {
    let mut knobs: Vec<_> = [PYTHON_VAR, TIMEOUT_VAR]
        .iter()
        .filter_map(|knob| Some(format!("{knob}={}", env::var(knob).ok()?)))
        .collect();
    if label != DEFAULT_LABEL {
        knobs.push(format!("{WIDTHS_VAR}={label}"));
    }
    knobs.push(format!("{MODULE_VAR}={module}"));
    format!("{} mise run imports", knobs.join(" "))
}
