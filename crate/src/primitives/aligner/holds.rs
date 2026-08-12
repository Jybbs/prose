//! Skip-hold and alignment-candidacy predicates for the alignment
//! rules: which rows a skip directive holds out of the column math, and
//! whether a run of members forms a valid alignment column.

use ruff_source_file::LineRanges;
use ruff_text_size::TextSize;

use super::{Member, members::baseline};
use crate::{rule::RuleId, source::Source};

/// Returns `true` when `members` form a multi-row group whose aligned
/// tokens sit on distinct source lines at a shared display-column
/// baseline, read from the source as written.
pub(crate) fn is_alignment_candidate(source: &Source, members: &[Member]) -> bool {
    shares_column(members, |m| baseline(source, m))
}

/// Returns `true` when the line containing `anchor` falls under a skip
/// directive for `rule`: a bare `# prose: skip` span, a `# prose: off`
/// region, or a `# prose: skip[<id>]` listing `rule`. A directive
/// trailing a wrapped statement covers every line that statement spans.
pub(crate) fn is_held(source: &Source, rule: RuleId, anchor: TextSize) -> bool {
    let suppression = source.suppression_map();
    suppression.has_format_suppression()
        && suppression.suppresses(source.text().full_line_range(anchor), rule)
}

/// Returns the rows of `members` whose anchor line is not skip-held for
/// `rule`, dropping the held rows so neighbors align around them.
pub(crate) fn retain_unheld(
    source: &Source,
    rule: RuleId,
    members: impl IntoIterator<Item = Member>,
) -> Vec<Member> {
    members
        .into_iter()
        .filter(|m| !is_held(source, rule, m.line_start))
        .collect()
}

/// Returns `true` when `members` form a multi-row group on distinct
/// source lines whose `baseline_of` columns match pairwise.
pub(crate) fn shares_column(
    members: &[Member],
    mut baseline_of: impl FnMut(Member) -> usize,
) -> bool {
    members.len() >= 2
        && members
            .windows(2)
            .all(|w| w[0].line_start != w[1].line_start && baseline_of(w[0]) == baseline_of(w[1]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{align_member, parse, range};

    #[test]
    fn is_alignment_candidate_holds_for_shared_baseline() {
        // Two `=` rows on distinct lines, each opening at column 0.
        let source = parse("ab = 1\ncd = 2\n");
        let members = [
            align_member(range(2, 3), 0, 2),
            align_member(range(9, 10), 7, 2),
        ];

        assert!(is_alignment_candidate(&source, &members));
    }

    #[test]
    fn is_alignment_candidate_rejects_differing_baselines() {
        // Distinct lines, but the `q.` prefix opens the second row two
        // columns right, so a shared `=` column would land where no row sits.
        let source = parse("ab = 1\nq.cd = 2\n");
        let members = [
            align_member(range(2, 3), 0, 2),
            align_member(range(11, 12), 7, 2),
        ];

        assert!(!is_alignment_candidate(&source, &members));
    }

    #[test]
    fn is_alignment_candidate_rejects_same_line() {
        // Two rows sharing a source line never form a column.
        let source = parse("ab = cd = 1\n");
        let members = [
            align_member(range(2, 3), 0, 2),
            align_member(range(7, 8), 0, 2),
        ];

        assert!(!is_alignment_candidate(&source, &members));
    }

    #[test]
    fn is_alignment_candidate_rejects_singleton() {
        let source = parse("ab = 1\n");

        assert!(!is_alignment_candidate(
            &source,
            &[align_member(range(2, 3), 0, 2)]
        ));
    }
}
