//! The padding `strip-stranded-padding` drops, read ahead of that rule
//! by a rule measuring a row at the width the pipeline settles it to.
//! [`Stranding`] names the padding rule so a row its skip directive
//! holds stays out of the prediction, [`Stranding::edits`] lists every
//! deletion and collapse the rule emits over a source, and [`slack`]
//! sums the columns those edits take off one span.

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    primitives::{
        aligner,
        colon_targets::ColonEmitter,
        tokens::{is_delimiter_padding, is_interpolated_string_start},
    },
    rule::RuleId,
    source::Source,
};

/// The padding rule a prediction reads, carried by rule id.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Stranding {
    rule: RuleId,
}

impl Stranding {
    pub(crate) fn new(rule: RuleId) -> Self {
        Self { rule }
    }

    /// Every edit the padding rule emits over `source`, ascending by
    /// start. A colon group no shared column justifies has its
    /// pre-colon gap cleared and its post-colon gap collapsed to one
    /// space, and the whitespace run directly inside a bracket
    /// delimiter is deleted.
    pub(crate) fn edits(self, source: &Source) -> Vec<Edit> {
        let mut emitter = Emitter {
            edits: Vec::new(),
            rule: self.rule,
            source,
        };
        emitter.walk(source);
        emitter.edits.extend(
            delimiter_padding_gaps(source, source.module_range()).map(Edit::range_deletion),
        );
        emitter.edits.sort_by_key(Ranged::start);
        emitter.edits
    }
}

struct Emitter<'a> {
    edits: Vec<Edit>,
    rule: RuleId,
    source: &'a Source,
}

impl ColonEmitter for Emitter<'_> {
    /// Clears the pre-colon gap and collapses the post-colon gap to one
    /// space for a group that is not an
    /// [`aligner::is_alignment_candidate`], so no shared column
    /// justifies the padding. A singleton has no neighbor row, a
    /// same-line group has no column distinction, and a distinct-line
    /// group whose rows open at differing baselines realizes no shared
    /// column. A distinct-line group at one baseline belongs to
    /// `align_colons` and emits nothing here. The pre-colon `width > 0`
    /// guard rejects the edge case where a `:` sits on its own indented
    /// line and the gap is leading indent rather than padding. The
    /// `value_gap` rewrite skips a value that opens on a later line.
    fn handle(&mut self, members: &[aligner::Member]) {
        if aligner::is_alignment_candidate(members) {
            return;
        }
        for m in members {
            if m.width > 0 {
                self.edits
                    .extend(aligner::space_padding_edit(self.source, m.gap, 0));
            }
            if let Some(gap) = m.rewritten_value_gap(self.source) {
                self.edits
                    .extend(aligner::space_padding_edit(self.source, gap, 1));
            }
        }
    }

    fn rule(&self) -> RuleId {
        self.rule
    }
}

/// The columns the edits in `edits` take off `range`, negative where
/// they widen it, counting each edit `range` covers whole. An insertion
/// at either boundary belongs to the text beside `range` and is left
/// out. `edits` is the ascending list [`Stranding::edits`] builds.
pub(crate) fn slack(source: &Source, edits: &[Edit], range: TextRange) -> isize {
    let first = edits.partition_point(|edit| edit.start() < range.start());
    edits[first..]
        .iter()
        .take_while(|edit| edit.start() <= range.end())
        .filter(|edit| {
            range.contains_range(edit.range())
                && !(edit.range().is_empty()
                    && (edit.start() == range.start() || edit.start() == range.end()))
        })
        .map(|edit| {
            source.slice(edit.range()).width().cast_signed()
                - edit
                    .content()
                    .map_or(0, UnicodeWidthStr::width)
                    .cast_signed()
        })
        .sum()
}

/// The whitespace runs inside `range` sitting directly inside a bracket
/// delimiter, after an opening `(` `[` `{` or before its closer, each
/// sharing a line with the neighbor it pads against. A closer on its
/// own line keeps its leading indent, since the gap then spans a line
/// break, and a run inside an f-string or t-string replacement field
/// stays untouched, tracked through `interp_depth`.
pub(crate) fn delimiter_padding_gaps(
    source: &Source,
    range: TextRange,
) -> impl Iterator<Item = TextRange> + '_ {
    let mut interp_depth: u32 = 0;
    source
        .tokens_overlapping(range)
        .tuple_windows()
        .filter_map(move |(token, next)| {
            let kind = token.kind();
            if is_interpolated_string_start(kind) {
                interp_depth += 1;
            } else if kind.is_interpolated_string_end() {
                interp_depth = interp_depth.saturating_sub(1);
            }
            let gap = TextRange::new(token.end(), next.start());
            (interp_depth == 0
                && !gap.is_empty()
                && range.contains_range(gap)
                && !source.contains_line_break(gap)
                && is_delimiter_padding(kind, next.kind()))
            .then_some(gap)
        })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::testing::{align_member, parse, range};

    fn run_strip(source: &Source, members: &[aligner::Member]) -> Vec<Edit> {
        let mut emitter = Emitter {
            edits: Vec::new(),
            rule: RuleId::from("strip-stranded-padding"),
            source,
        };
        emitter.handle(members);
        emitter.edits
    }

    /// Every delimiter-padding gap across `src`.
    fn delimiter_gaps(src: &str) -> Vec<TextRange> {
        let source = parse(src);
        delimiter_padding_gaps(&source, source.module_range()).collect()
    }

    #[test]
    fn delimiter_skips_closer_on_its_own_line() {
        // The closer carries leading indent rather than interior padding
        // once a line break separates it from the content.
        assert!(delimiter_gaps("x = [\n    1\n    ]\n").is_empty());
    }

    #[rstest]
    fn delimiter_skips_interpolated_replacement_field(
        #[values(
            "v = f\"{ x }\"\n",
            "v = f\"{ x = }\"\n",
            "v = t\"{ x }\"\n",
            "v = t\"{ x = }\"\n"
        )]
        src: &str,
    ) {
        // A debug `f"{ x = }"` or t-string echoes its interior spaces, so
        // the replacement-field braces are left untouched.
        assert!(delimiter_gaps(src).is_empty());
    }

    #[test]
    fn delimiter_skips_padding_before_a_comment() {
        // Padding between an opener and a same-line comment is left, so the
        // comment does not fuse onto the bracket.
        assert!(delimiter_gaps("f(  # note\n    a,\n)\n").is_empty());
    }

    #[test]
    fn delimiter_strips_after_opener_and_before_closer() {
        assert_eq!(delimiter_gaps("f( 1 )\n"), [range(2, 3), range(4, 5)]);
    }

    #[test]
    fn delimiter_strips_empty_pair_once() {
        // Both sides of `f( )` qualify, yet the lone gap emits a single
        // edit rather than an overlapping pair.
        assert_eq!(delimiter_gaps("f( )\n"), [range(2, 3)]);
    }

    #[rstest]
    #[case::delimiter_padding_narrows("x = [ 1, 2 ]\n", 4, 12, 2)]
    #[case::colon_gap_collapse_widens("x = {'a':1}\n", 4, 11, -1)]
    #[case::pre_colon_gap_clears("x = {'a'  : 1}\n", 4, 14, 2)]
    #[case::edit_outside_the_span("x = [ 1, 2 ]\n", 0, 4, 0)]
    #[case::insertion_at_the_boundary("x = {'a':1}\n", 9, 10, 0)]
    fn slack_sums_the_columns_the_edits_take_off_a_span(
        #[case] src: &str,
        #[case] start: u32,
        #[case] end: u32,
        #[case] expected: isize,
    ) {
        let source = parse(src);
        let edits = Stranding::new(RuleId::from("strip-stranded-padding")).edits(&source);
        assert_eq!(slack(&source, &edits, range(start, end)), expected);
    }

    #[test]
    fn strip_handles_empty_members_slice() {
        assert!(run_strip(&parse(""), &[]).is_empty());
    }

    #[test]
    fn strip_leaves_a_value_gap_that_crosses_a_line_break() {
        // A colon whose value opens on a later line keeps its placement,
        // so the pre-colon padding strips whereas the post-colon gap is
        // not collapsed across the break.
        let source = parse("d = {\"k\"  :\n    v}\n");
        let member =
            align_member(range(8, 10), 0, 3).with_value_gap(TextSize::of(':'), TextSize::new(16));
        let edits = run_strip(&source, &[member]);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range(), range(8, 10));
    }

    #[test]
    fn strip_skips_multi_member_groups_on_distinct_lines() {
        // Both rows open at a column-0 baseline, so the distinct-line
        // group stays a candidate and passes through to `align_colons`.
        let source = parse("ab: 1\ncd: 2\n");
        let members = [
            align_member(range(2, 2), 0, 2),
            align_member(range(8, 8), 6, 2),
        ];
        assert!(run_strip(&source, &members).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_empty_gap() {
        assert!(run_strip(&parse(""), &[align_member(range(0, 0), 0, 0)]).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_indent_gap() {
        assert!(run_strip(&parse("x: 1\n"), &[align_member(range(0, 4), 0, 0)]).is_empty());
    }

    #[test]
    fn strip_strips_every_member_when_colons_share_a_line() {
        let source = parse("{x: 1, y: 2}\n");
        let members = [
            align_member(range(3, 5), 0, 3),
            align_member(range(8, 10), 0, 5),
        ];
        assert_eq!(run_strip(&source, &members).len(), 2);
    }

    #[test]
    fn strip_strips_multi_member_groups_at_differing_baselines() {
        // Distinct lines opening at different indents (free inside the
        // brackets), so the `:`s share no column and the pre-`:` padding
        // strips the way a singleton's does.
        let source = parse("d = {\n    \"ab\"  : 1,\n        \"cd\"  : 2,\n}\n");
        let members = [
            align_member(range(14, 16), 6, 4),
            align_member(range(33, 35), 21, 4),
        ];
        assert_eq!(run_strip(&source, &members).len(), 2);
    }

    #[test]
    fn strip_strips_singleton_with_content_and_gap() {
        let edits = run_strip(&parse("abc  : 1\n"), &[align_member(range(3, 5), 0, 3)]);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start(), TextSize::new(3));
        assert_eq!(edits[0].end(), TextSize::new(5));
    }
}
