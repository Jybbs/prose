//! Edit-shaping primitives shared across rules. `apply_edits_mapped`
//! splices a sorted edit list into a source string, the pipeline
//! runner's transform between rules, pairing that string with a
//! `SourceMap` of one marker per applied edit. `apply_inline_edits`
//! folds a list of edits into a source range, returning `Cow::Borrowed`
//! when no edit applies. Both decline overlapping edits,
//! `apply_edits_mapped` with `None` and `apply_inline_edits` with
//! `Cow::Borrowed`.
//! `narrowed_replacement` trims a candidate replacement to its minimal
//! divergent range against the source, and `insert_edit` keeps a rule's own
//! accumulator in that sorted order as it emits. The `forward_*`
//! functions move an offset, a range, an edit list, or a notebook's
//! cell boundaries through the `SourceMap` of an applied edit set, and
//! `shifted_past` reads the same map for a boundary no edit replaced,
//! the slide the reparse between rules carries over every held range.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{primitives::sorted_slot, source::Source};

mod apply;
mod offsets;

pub(crate) use apply::{apply_edits_mapped, apply_inline_edits, splice_bodies};
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

/// `text` carrying a leading space where the character before `start`
/// would otherwise run into it, as `return[x for x in xs]` does.
pub(crate) fn padded(source: &Source, start: TextSize, text: String) -> String {
    if text.starts_with(joins_an_identifier) && joins_before(source, start) {
        format!(" {text}")
    } else {
        text
    }
}

/// True where the character ahead of `offset` joins an identifier, so
/// text opening with one placed there runs into it.
pub(crate) fn joins_before(source: &Source, offset: TextSize) -> bool {
    source.text()[..offset.to_usize()]
        .chars()
        .next_back()
        .is_some_and(joins_an_identifier)
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

/// The edit clearing every full line `range` sits on, its final line
/// terminator included, held back from the newline closing a notebook
/// cell so the deletion empties that cell rather than merging it into
/// the next.
pub(crate) fn whole_line_deletion(source: &Source, range: TextRange) -> Edit {
    Edit::range_deletion(source.full_lines_within_cell(range))
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
    use crate::testing::{parse, range};

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

    #[test]
    fn whole_line_deletion_clears_through_the_line_terminator() {
        let source = parse("import os\nimport sys\nx = 1\n");
        let edit = whole_line_deletion(&source, range(10, 16));

        assert_eq!(edit.range(), range(10, 21));
        assert_eq!(edit.content(), None);
    }
}
