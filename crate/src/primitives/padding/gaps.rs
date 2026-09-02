//! The whitespace runs inside a bracket delimiter and the columns an
//! edit set takes off a span.

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};

use super::*;
use crate::primitives::inline::display_width;

/// The whitespace runs inside `range` sitting directly inside a bracket
/// delimiter, after an opening `(` `[` `{` or before its closer, each
/// sharing a line with the neighbor it pads against. A closer on its
/// own line keeps its leading indent, since the gap then spans a line
/// break, and a run inside an f-string or t-string replacement field
/// stays untouched, tracked through `interp_depth`.
pub(super) fn delimiter_padding_gaps(
    source: &Source,
    range: TextRange,
) -> impl Iterator<Item = TextRange> + '_ {
    let mut interp_depth = source.interpolation_depth_at(range.start());
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

/// The display width of every [`delimiter_padding_gaps`] run inside
/// `range`, which is the width `strip-stranded-padding` takes off it.
pub(crate) fn delimiter_padding_width(source: &Source, range: TextRange) -> usize {
    delimiter_padding_gaps(source, range)
        .map(|gap| display_width(source.slice(gap)))
        .sum()
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
            display_width(source.slice(edit.range())).cast_signed()
                - edit.content().map_or(0, display_width).cast_signed()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{parse, range};

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
        let edits = Stranding::new(RuleId::from("strip-stranded-padding"), true).edits(&source);
        assert_eq!(slack(&source, &edits, range(start, end)), expected);
    }
}
