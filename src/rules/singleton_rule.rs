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

use crate::config::Config;
use crate::primitives::aligner;
use crate::primitives::colon_targets::ColonEmitter;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct SingletonRule;

impl SingletonRule {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for SingletonRule {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut emitter = Emitter { edits: Vec::new() };
        emitter.walk(source);
        emitter.edits
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Emitter {
    edits: Vec<Edit>,
}

impl ColonEmitter for Emitter {
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
        if aligner::is_alignment_candidate(members) {
            return;
        }
        self.edits.extend(
            members
                .iter()
                .filter(|m| m.width > 0 && !m.gap.is_empty())
                .map(|m| Edit::range_deletion(m.gap)),
        );
    }
}

#[cfg(test)]
mod tests {
    use ruff_text_size::{Ranged, TextRange, TextSize};

    use super::*;

    fn run_strip(members: &[aligner::Member]) -> Vec<Edit> {
        let mut emitter = Emitter { edits: Vec::new() };
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
                gap: TextRange::new(TextSize::new(2), TextSize::new(2)),
                line_start: TextSize::new(0),
                width: 2,
            },
            aligner::Member {
                gap: TextRange::new(TextSize::new(8), TextSize::new(8)),
                line_start: TextSize::new(6),
                width: 2,
            },
        ];
        assert!(run_strip(&members).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_empty_gap() {
        let member = aligner::Member {
            gap: TextRange::new(TextSize::new(0), TextSize::new(0)),
            line_start: TextSize::new(0),
            width: 0,
        };
        assert!(run_strip(&[member]).is_empty());
    }

    #[test]
    fn strip_skips_zero_width_member_with_indent_gap() {
        let member = aligner::Member {
            gap: TextRange::new(TextSize::new(0), TextSize::new(4)),
            line_start: TextSize::new(0),
            width: 0,
        };
        assert!(run_strip(&[member]).is_empty());
    }

    #[test]
    fn strip_strips_every_member_when_colons_share_a_line() {
        let members = [
            aligner::Member {
                gap: TextRange::new(TextSize::new(3), TextSize::new(5)),
                line_start: TextSize::new(0),
                width: 3,
            },
            aligner::Member {
                gap: TextRange::new(TextSize::new(8), TextSize::new(10)),
                line_start: TextSize::new(0),
                width: 5,
            },
        ];
        assert_eq!(run_strip(&members).len(), 2);
    }

    #[test]
    fn strip_strips_singleton_with_content_and_gap() {
        let member = aligner::Member {
            gap: TextRange::new(TextSize::new(3), TextSize::new(5)),
            line_start: TextSize::new(0),
            width: 3,
        };
        let edits = run_strip(&[member]);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start(), TextSize::new(3));
        assert_eq!(edits[0].end(), TextSize::new(5));
    }
}
