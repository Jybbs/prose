//! Edit-shaping primitives shared across rules. `apply_edits_mapped`
//! splices sorted edits into a string beside a `SourceMap` of one
//! start-and-end marker per edit, and `apply_inline_edits` folds edits
//! into a source range, the two declining an overlap with `None` and
//! `Cow::Borrowed` in turn. `narrowed_replacement` trims a replacement
//! to the range that differs, `insert_edit` keeps an accumulator sorted
//! by start, the `forward_*` functions move an offset, a range, or cell
//! boundaries through a `SourceMap`, `shifted_past` reads one for a
//! boundary no edit replaced, and `whole_line_deletions` clears dropped
//! statements, keeping one blank run where two would meet.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{primitives::sorted_slot, source::Source};

mod apply;
mod deletions;
mod offsets;

pub(crate) use apply::{apply_edits_mapped, apply_inline_edits, splice_bodies};
pub(crate) use deletions::whole_line_deletions;
pub(crate) use offsets::{
    forward_offsets, forward_range, forward_start, narrowed_replacement, shifted_past,
};

/// True when any element of `parts` is `Cow::Owned`, the signal a
/// rewrite produced fresh content rather than a borrow of the source.
pub(crate) fn any_owned(parts: &[Cow<str>]) -> bool {
    parts.iter().any(|part| matches!(part, Cow::Owned(_)))
}

/// Inserts `edit` at the slot keeping `edits` ascending by start, the
/// order [`apply_inline_edits`] reads them in.
pub(crate) fn insert_edit(edits: &mut Vec<Edit>, edit: Edit) {
    let slot = sorted_slot(edits, &edit, Ranged::start);
    edits.insert(slot, edit);
}

/// True where `c` is an identifier character: a letter, a digit, or an
/// underscore.
pub(crate) fn joins_an_identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// True where the character ahead of `offset` joins an identifier, so
/// text opening with one placed there runs into it.
pub(crate) fn joins_before(source: &Source, offset: TextSize) -> bool {
    source.text()[..offset.to_usize()]
        .chars()
        .next_back()
        .is_some_and(joins_an_identifier)
}

/// `text` carrying a leading space where the character before `start`
/// would otherwise run into it, as `return[x for x in xs]` does.
pub(crate) fn padded(source: &Source, start: TextSize, text: String) -> String {
    if text.starts_with(joins_an_identifier) && joins_before(source, start) {
        format!(" {text}")
    } else {
        text
    }
}

/// The text ahead of `offset` on its logical line, clipped to `floor`
/// and rendered with `edits` applied.
pub(crate) fn placed_head<'a>(
    source: &'a Source,
    edits: &[Edit],
    offset: TextSize,
    floor: TextSize,
) -> Cow<'a, str> {
    let start = source.logical_line_start(offset).start().max(floor);
    apply_inline_edits(source, TextRange::new(start, offset), edits)
}

/// The edit rewriting `range` to `n` copies of `unit`, a deletion when
/// `n` is zero.
pub(crate) fn repeat_edit(range: TextRange, unit: &str, n: usize) -> Edit {
    replacement_or_deletion(range, unit.repeat(n))
}

/// Wraps each edit in its own single-edit fix group, the shape a rule
/// whose edits are mutually independent returns from `apply`.
pub(crate) fn singleton_groups(edits: impl IntoIterator<Item = Edit>) -> Vec<Vec<Edit>> {
    edits.into_iter().map(|edit| vec![edit]).collect()
}

fn replacement_or_deletion(range: TextRange, content: String) -> Edit {
    if content.is_empty() {
        Edit::range_deletion(range)
    } else {
        Edit::range_replacement(content, range)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case(6, "dict(", " dict(")]
    #[case(7, "dict(", "dict(")]
    #[case(6, "{", "{")]
    #[case(0, "dict(", "dict(")]
    fn padded_spaces_a_replacement_only_where_the_two_would_merge(
        #[case] start: u32,
        #[case] text: &str,
        #[case] expected: &str,
    ) {
        let source = parse("return [x for x in xs]\n");
        assert_eq!(
            padded(&source, TextSize::new(start), text.to_owned()),
            expected,
        );
    }
}
