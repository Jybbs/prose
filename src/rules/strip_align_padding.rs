//! Strips pre-`:` padding from the colon groups `align_colons` leaves
//! unaligned, where the gap before the `:` has no column to align to.
//! Runs after the alignment rules in `Pipeline::with_defaults` so it
//! sees their output.

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
    /// Strips each member's gap when the group is not an
    /// [alignment candidate](aligner::is_alignment_candidate), so the
    /// groups `align_colons` owns emit nothing here. The `width > 0`
    /// guard skips a `:` on its own indented line, where the gap is
    /// leading indent rather than padding.
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

    fn run_strip(members: &[aligner::Member]) -> Vec<Edit> {
        // Two flush lines starting at offsets 0 and 6 back the synthetic
        // members' `line_start`s, so the baseline read sees one shared
        // indent.
        let source = parse("aa: 0\nbb: 1\n");
        let mut emitter = Emitter {
            edits: Vec::new(),
            source: &source,
        };
        emitter.handle(members);
        emitter.edits
    }

    #[test]
    fn strip_handles_empty_members_slice() {
        assert!(run_strip(&[]).is_empty());
    }

    #[test]
    fn strip_skips_multi_member_groups_on_distinct_lines() {
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
        assert!(run_strip(&members).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_empty_gap() {
        let member = aligner::Member {
            gap: range(0, 0),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 0,
        };
        assert!(run_strip(&[member]).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_indent_gap() {
        let member = aligner::Member {
            gap: range(0, 4),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 0,
        };
        assert!(run_strip(&[member]).is_empty());
    }

    #[test]
    fn strip_strips_every_member_when_colons_share_a_line() {
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
        assert_eq!(run_strip(&members).len(), 2);
    }

    #[test]
    fn strip_strips_singleton_with_content_and_gap() {
        let member = aligner::Member {
            gap: range(3, 5),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 3,
        };
        let edits = run_strip(&[member]);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start(), TextSize::new(3));
        assert_eq!(edits[0].end(), TextSize::new(5));
    }
}
