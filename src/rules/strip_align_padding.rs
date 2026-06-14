//! Strips the pre-`:` padding on aligned contexts whose `:`s have no
//! column to align to. The two cases are a singleton group (one
//! member, so no neighbor row) and a multi-member group whose `:`s
//! all share a source line (no column distinction across rows). Both
//! reduce to "alignment is not happening here," at which point the
//! pre-`:` gap is visual noise and the rule strips it. Multi-member
//! groups whose `:`s sit on distinct lines belong to `align_colons`
//! and pass through this rule untouched. Runs after the alignment
//! rules in `Pipeline::with_defaults` so it sees their output.

use ruff_diagnostics::Edit;

use crate::{
    config::Config,
    primitives::{aligner, colon_targets::ColonEmitter, edit::singleton_groups},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct StripAlignPadding;

impl StripAlignPadding {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for StripAlignPadding {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut emitter = Emitter {
            edits: Vec::new(),
            source,
        };
        emitter.walk(source);
        singleton_groups(emitter.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Emitter<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl ColonEmitter for Emitter<'_> {
    /// Emits a deletion edit per member when alignment is not
    /// happening for the group. Singleton groups always qualify, since
    /// a single row has nothing to align against. Multi-member groups
    /// qualify when their `:`s share a source line, since no column
    /// distinguishes the rows. Multi-member groups on distinct lines
    /// belong to `align_colons` and emit nothing here. The `width > 0`
    /// guard rejects the edge case where a `:` sits on its own
    /// indented line and the "gap" is leading indent rather than
    /// padding.
    fn handle(&mut self, members: &[aligner::Member]) {
        if aligner::is_alignment_candidate(self.source, members) {
            return;
        }
        self.edits.extend(
            members
                .iter()
                .filter(|m| m.width > 0 && !m.gap.is_empty())
                .map(|m| Edit::range_deletion(m.gap)),
        );
    }

    fn rule(&self) -> RuleId {
        StripAlignPadding::SLUG
    }
}

#[cfg(test)]
mod tests {
    use ruff_text_size::{Ranged, TextSize};

    use super::*;
    use crate::testing::{parse, range};

    fn run_strip(source: &Source, members: &[aligner::Member]) -> Vec<Edit> {
        let mut emitter = Emitter {
            edits: Vec::new(),
            source,
        };
        emitter.handle(members);
        emitter.edits
    }

    #[test]
    fn strip_handles_empty_members_slice() {
        assert!(run_strip(&parse(""), &[]).is_empty());
    }

    #[test]
    fn strip_skips_multi_member_groups_on_distinct_lines() {
        // Both rows open at a column-0 baseline, so the distinct-line
        // group stays a candidate and passes through to `align_colons`.
        let source = parse("ab: 1\ncd: 2\n");
        let members = [
            aligner::Member {
                gap: range(2, 2),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 2,
            },
            aligner::Member {
                gap: range(8, 8),
                line_start: TextSize::new(6),
                op_width: 0,
                width: 2,
            },
        ];
        assert!(run_strip(&source, &members).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_empty_gap() {
        let member = aligner::Member {
            gap: range(0, 0),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 0,
        };
        assert!(run_strip(&parse(""), &[member]).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_indent_gap() {
        let member = aligner::Member {
            gap: range(0, 4),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 0,
        };
        assert!(run_strip(&parse("x: 1\n"), &[member]).is_empty());
    }

    #[test]
    fn strip_strips_every_member_when_colons_share_a_line() {
        let source = parse("{x: 1, y: 2}\n");
        let members = [
            aligner::Member {
                gap: range(3, 5),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 3,
            },
            aligner::Member {
                gap: range(8, 10),
                line_start: TextSize::new(0),
                op_width: 0,
                width: 5,
            },
        ];
        assert_eq!(run_strip(&source, &members).len(), 2);
    }

    #[test]
    fn strip_strips_singleton_with_content_and_gap() {
        let member = aligner::Member {
            gap: range(3, 5),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 3,
        };
        let edits = run_strip(&parse("abc  : 1\n"), &[member]);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start(), TextSize::new(3));
        assert_eq!(edits[0].end(), TextSize::new(5));
    }
}
