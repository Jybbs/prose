//! Aligns the `#` of a trailing comment across a run of consecutive
//! source lines, seating each comment in the run at one shared column
//! the [trailing gap](TRAILING_GAP) past the widest row's code. A line
//! carrying no trailing comment, a blank line, and an own-line comment
//! each break the run, and a run whose rows open at differing indents
//! aligns nothing. A row reaching no shared column collapses to the
//! trailing gap itself, covering a lone comment, a run at differing
//! indents, and a row partitioning out on the `max-shift` or
//! line-length cap.

use ruff_diagnostics::Edit;
use ruff_python_trivia::CommentRanges;

use crate::{
    config::Config,
    primitives::{aligner, comments::TRAILING_GAP},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct AlignComments {
    settings: aligner::Settings,
}

impl AlignComments {
    pub(crate) const MESSAGE: &'static str = "align consecutive trailing comments";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.align_comments)
                .with_buffer(TRAILING_GAP.len())
                .within(
                    config.code_width(),
                    config.stranded_padding(),
                    config.comment_settling(),
                ),
        }
    }
}

impl Rule for AlignComments {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut walker = aligner::AlignWalker::new(source, self.settings, Self::SLUG);
        for group in trailing_comment_groups(source, walker.rule) {
            walker.emit_group_or_buffer(&group);
        }
        walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The runs of trailing comments sitting on consecutive source lines,
/// each row anchored on its `#` and measured by the code ahead of it. A
/// row held for `rule` bridges its run, leaving the neighbors on either
/// side to align as one block.
fn trailing_comment_groups(source: &Source, rule: RuleId) -> Vec<Vec<aligner::Member>> {
    let trailing = source
        .comment_ranges()
        .into_iter()
        .filter(|r| !CommentRanges::is_own_line(r.start(), source.text()));
    aligner::adjacent_member_groups(source, trailing, false, |range| {
        aligner::line_anchored_member(source, range.start()).slot(source, rule)
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn trailing_comment_groups_measures_the_code_ahead_of_the_hash() {
        let source = parse("x = 1  # a\n");
        let groups = trailing_comment_groups(&source, AlignComments::SLUG);
        assert_eq!(groups[0][0].width, "x = 1".len());
    }

    #[rstest]
    #[case::adjacent_rows("x = 1  # a\nlonger = 2  # b\n", vec![2])]
    #[case::own_line_comment("x = 1  # a\n# divider\ny = 2  # b\n", vec![1, 1])]
    #[case::blank_line("x = 1  # a\n\ny = 2  # b\n", vec![1, 1])]
    #[case::held_row_bridges(
        "x = 1  # a\ny = 2  # prose: skip[align-comments]\nz = 3  # c\n",
        vec![2]
    )]
    #[case::own_line_only("# lead\n# follow\n", vec![])]
    fn trailing_comment_groups_partitions_by_adjacency(
        #[case] src: &str,
        #[case] expected: Vec<usize>,
    ) {
        let source = parse(src);
        let groups = trailing_comment_groups(&source, AlignComments::SLUG);
        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), expected);
    }
}
