//! The diff between a module's two versions, meaning the original rows a
//! formatted row came from and the hunk around a row.

use std::ops::Range;

use itertools::Itertools;
use similar::{DiffTag, TextDiff};

/// How many diff lines a hunk shows either side of the row it centres on.
const CONTEXT: usize = 3;

/// The unified-diff lines around one row, cut to [`CONTEXT`] lines either
/// side with an ellipsis marking each cut.
///
/// Where the row is unknown the window centres on the first changed line
/// naming `name`, and failing that on the first changed line at all.
pub(crate) fn hunk(before: &[&str], after: &[&str], row: Option<usize>, name: &str) -> Vec<String> {
    let mut shown: Vec<(Option<usize>, String)> = Vec::new();
    for op in TextDiff::from_slices(before, after).ops() {
        let (tag, old, new) = op.as_tag_tuple();
        if matches!(tag, DiffTag::Delete | DiffTag::Replace) {
            shown.extend(before[old].iter().map(|line| (None, format!("-{line}"))));
        }
        if tag != DiffTag::Delete {
            let mark = if tag == DiffTag::Equal { ' ' } else { '+' };
            shown.extend(
                after[new.clone()]
                    .iter()
                    .enumerate()
                    .map(|(k, line)| (Some(new.start + k), format!("{mark}{line}"))),
            );
        }
    }
    let index = match row {
        Some(row) => shown
            .iter()
            .position(|(seen, _)| *seen == Some(row - 1))
            .unwrap_or_default(),
        None => {
            let changed: Vec<_> = shown
                .iter()
                .positions(|(_, line)| !line.starts_with(' '))
                .collect();
            changed
                .iter()
                .find(|at| !name.is_empty() && shown[**at].1.contains(name))
                .or_else(|| changed.first())
                .copied()
                .unwrap_or_default()
        }
    };
    let low = index.saturating_sub(CONTEXT);
    let high = (index + CONTEXT + 1).min(shown.len());
    let opening = (low > 0).then(|| "...".to_owned());
    let closing = (high < shown.len()).then(|| "...".to_owned());
    opening
        .into_iter()
        .chain(shown[low..high].iter().map(|(_, line)| line.clone()))
        .chain(closing)
        .collect()
}

/// The original rows that one row of the formatted side came from, which is
/// one row for an unchanged block, every row of the block that rewrote it,
/// and the row an insertion landed at.
pub(crate) fn mapped_rows(before: &[&str], after: &[&str], row: usize) -> Range<usize> {
    for op in TextDiff::from_slices(before, after).ops() {
        let (tag, old, new) = op.as_tag_tuple();
        if new.contains(&(row - 1)) {
            if tag == DiffTag::Equal {
                let landed = old.start + row - new.start;
                return landed..landed + 1;
            }
            return old.start + 1..(old.start + 1).max(old.end) + 1;
        }
    }
    0..0
}
