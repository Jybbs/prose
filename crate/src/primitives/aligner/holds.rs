//! Skip-hold and alignment-candidacy predicates for the alignment
//! rules: which rows a skip directive holds out of the column math, and
//! whether a run of members forms a valid alignment column.

use ruff_source_file::LineRanges;
use ruff_text_size::TextSize;

use super::{Member, members::baseline};
use crate::{rule::RuleId, source::Source};

/// Returns `true` when `members` form a multi-row group whose aligned
/// tokens sit on distinct source lines at a shared display-column
/// baseline.
pub(crate) fn is_alignment_candidate(source: &Source, members: &[Member]) -> bool {
    members.len() >= 2
        && members.windows(2).all(|w| {
            w[0].line_start != w[1].line_start && baseline(source, w[0]) == baseline(source, w[1])
        })
}

/// Returns `true` when the line containing `anchor` carries a skip
/// directive for `rule`: a bare `# prose: skip` / `# fmt: skip` span, a
/// `# fmt: off` region, or `# prose: skip[rule]`. A held row stays out
/// of the column math and emits no edit, so its neighbors align around
/// it. Short-circuits when the source carries no format suppression.
pub(crate) fn is_held(source: &Source, rule: RuleId, anchor: TextSize) -> bool {
    let suppression = source.suppression_map();
    if !suppression.has_format_suppression() && !suppression.has_skip_suppression() {
        return false;
    }
    suppression.intersects(source.text().full_line_range(anchor))
        || suppression.is_format_suppressed_at(source.line_index(anchor), rule)
}

/// Returns the rows of `members` whose anchor line is not skip-held for
/// `rule`, dropping the held rows so neighbors align around them.
/// `line_start` yields each row's anchor line, so a row type wrapping a
/// `Member` filters by the same line the member carries.
pub(crate) fn retain_unheld<M>(
    source: &Source,
    rule: RuleId,
    members: impl IntoIterator<Item = M>,
    line_start: impl Fn(&M) -> TextSize,
) -> Vec<M> {
    members
        .into_iter()
        .filter(|m| !is_held(source, rule, line_start(m)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{parse, range};

    #[test]
    fn is_alignment_candidate_holds_for_shared_baseline() {
        // Two `=` rows on distinct lines, each opening at column 0.
        let source = parse("ab = 1\ncd = 2\n");
        let members = [
            Member {
                gap: range(2, 3),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 2,
            },
            Member {
                gap: range(9, 10),
                line_start: TextSize::new(7),
                op_width: 0,
                width: 2,
            },
        ];

        assert!(is_alignment_candidate(&source, &members));
    }

    #[test]
    fn is_alignment_candidate_rejects_differing_baselines() {
        // Distinct lines, but the `q.` prefix opens the second row two
        // columns right, so a shared `=` column would land where no row sits.
        let source = parse("ab = 1\nq.cd = 2\n");
        let members = [
            Member {
                gap: range(2, 3),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 2,
            },
            Member {
                gap: range(11, 12),
                line_start: TextSize::new(7),
                op_width: 0,
                width: 2,
            },
        ];

        assert!(!is_alignment_candidate(&source, &members));
    }

    #[test]
    fn is_alignment_candidate_rejects_same_line() {
        // Two rows sharing a source line never form a column.
        let source = parse("ab = cd = 1\n");
        let members = [
            Member {
                gap: range(2, 3),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 2,
            },
            Member {
                gap: range(7, 8),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 2,
            },
        ];

        assert!(!is_alignment_candidate(&source, &members));
    }

    #[test]
    fn is_alignment_candidate_rejects_singleton() {
        let source = parse("ab = 1\n");
        let members = [Member {
            gap: range(2, 3),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 2,
        }];

        assert!(!is_alignment_candidate(&source, &members));
    }
}
