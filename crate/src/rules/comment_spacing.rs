//! Settles every comment to one space between its leading hash run and
//! its text, and widens the gap before a trailing comment to at least
//! two spaces. A space-opened run keeps whatever width it carries where
//! the comment opens at a column an adjacent comment shares, so a block
//! laid out in one column holds its indents. The opener passes through
//! where the run is followed by `!`, `'`, `:`, or `|`, and where no text
//! follows the run at all, leaving the trailing gap settled either way.

use ruff_diagnostics::Edit;
use ruff_python_trivia::{CommentRanges, PythonWhitespace};
use ruff_text_size::{TextLen, TextRange};

use crate::{
    config::Config,
    primitives::{
        aligner::{line_gap_before, space_padding_edit},
        comments::TRAILING_GAP,
    },
    rule::{Rule, RuleId},
    source::Source,
};

/// The characters whose appearance directly after a comment's hash run
/// leaves the opener untouched, covering the shebang, the quoted and
/// piped forms, and the structured `#:` attribute-doc marker.
const EXEMPT_LEADERS: [char; 4] = ['!', '\'', ':', '|'];

pub(crate) struct CommentSpacing;

impl CommentSpacing {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for CommentSpacing {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let columnar = columnar_runs(source);
        source
            .comment_ranges()
            .into_iter()
            .zip(columnar)
            .filter_map(|(range, columnar)| {
                let edits: Vec<Edit> = gap_edit(source, range)
                    .into_iter()
                    .chain(opener_edit(source, range, columnar))
                    .collect();
                (!edits.is_empty()).then_some(edits)
            })
            .collect()
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// One flag per comment in source order, `true` where the comment opens
/// at the same column as a comment on the line directly above or below
/// it. A bare `#` sustains a run, since it opens at that column too.
fn columnar_runs(source: &Source) -> Vec<bool> {
    let ranges = source.comment_ranges();
    let mut flags = vec![false; ranges.len()];
    for (i, pair) in ranges.windows(2).enumerate() {
        let [above, below] = [pair[0].start(), pair[1].start()];
        if source.consecutive_lines(above, below)
            && source.column_of(above) == source.column_of(below)
        {
            flags[i] = true;
            flags[i + 1] = true;
        }
    }
    flags
}

/// The edit widening the run of whitespace ahead of the trailing comment
/// at `range` to [`TRAILING_GAP`]. `None` for an own-line comment and
/// for a gap already that wide.
fn gap_edit(source: &Source, range: TextRange) -> Option<Edit> {
    if CommentRanges::is_own_line(range.start(), source.text()) {
        return None;
    }
    let gap = line_gap_before(source, range.start());
    if gap.len() >= TRAILING_GAP.text_len() {
        return None;
    }
    space_padding_edit(source, gap, TRAILING_GAP.len())
}

/// The edit settling the whitespace between the hash run of the comment
/// at `range` and its text to one space, or to none where no text
/// follows the run. A `columnar` comment carrying a space-opened run
/// keeps it at whatever width it holds, so an indented line inside a
/// comment block keeps its shape. `None` for an exempt leader and for a
/// run already reading that way.
fn opener_edit(source: &Source, range: TextRange, columnar: bool) -> Option<Edit> {
    let body = source.slice(range).trim_start_matches('#');
    if body.starts_with(EXEMPT_LEADERS) {
        return None;
    }
    let text = body.trim_whitespace_start();
    if columnar && body.starts_with(' ') && !text.is_empty() {
        return None;
    }
    let opener = TextRange::at(
        range.end() - body.text_len(),
        body.text_len() - text.text_len(),
    );
    space_padding_edit(source, opener, usize::from(!text.is_empty()))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn apply_bundles_a_gap_and_an_opener_into_one_group() {
        // Both edits settle the same comment, so they arrive as one fix.
        let groups = CommentSpacing.apply(&parse("x = 1 #note\n"));
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn apply_keeps_each_comment_in_its_own_group() {
        let groups = CommentSpacing.apply(&parse("#one\n#two\n"));
        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), [1, 1]);
    }

    #[test]
    fn columnar_runs_carries_a_run_across_a_bare_hash() {
        // The bare `#` opens at the run's column too, so it sustains the
        // run rather than splitting it into two lone comments.
        let flags = columnar_runs(&parse("# lead\n#\n#     deep\n"));
        assert_eq!(flags, [true, true, true]);
    }

    #[rstest]
    #[case("#   lone\n", [false])]
    #[case("x = 1  # a\ny = 2  # b\n", [true, true])]
    #[case("hello = 1 # a\nhi = 2 #    b\n", [false, false])]
    fn columnar_runs_flags_only_a_shared_column_on_adjacent_lines<const N: usize>(
        #[case] src: &str,
        #[case] expected: [bool; N],
    ) {
        assert_eq!(columnar_runs(&parse(src)), expected);
    }

    #[test]
    fn gap_edit_inserts_where_the_comment_abuts_its_code() {
        let source = parse("x = 1#note\n");
        let edit = gap_edit(&source, source.comment_ranges()[0]).expect("a zero gap widens");
        assert!(edit.range().is_empty());
        assert_eq!(edit.content(), Some("  "));
    }
}
