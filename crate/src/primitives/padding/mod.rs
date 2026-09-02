//! The padding `strip-stranded-padding` drops, read ahead of that rule
//! by a rule measuring a row at the width the pipeline settles it to.
//! [`Stranding`] names the padding rule and whether it runs, so a row
//! its skip directive holds stays out of the prediction and a disabled
//! rule predicts nothing, [`Stranding::edits`] lists every deletion and
//! collapse the rule emits over a source, and [`slack`] sums the
//! columns those edits take off one span.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange};

mod gaps;

use gaps::delimiter_padding_gaps;
pub(crate) use gaps::{delimiter_padding_width, slack};

use crate::{
    primitives::{
        aligner,
        colon_targets::ColonEmitter,
        range::covers,
        tokens::{is_delimiter_padding, is_interpolated_string_start},
    },
    rules::RuleId,
    source::Source,
};

/// The padding rule a prediction reads, carried by rule id beside
/// whether the rule runs at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Stranding {
    enabled: bool,
    rule: RuleId,
}

impl Stranding {
    pub(crate) fn new(rule: RuleId, enabled: bool) -> Self {
        Self { enabled, rule }
    }

    /// Every edit the padding rule emits over `source`, ascending by
    /// start, none where the rule is off. A colon group no shared
    /// column justifies has its pre-colon gap cleared and its post-colon
    /// gap collapsed to one space, and the whitespace run directly
    /// inside a bracket delimiter is deleted, a row the rule's skip
    /// directive holds keeping every gap it carries.
    pub(crate) fn edits(self, source: &Source) -> Vec<Edit> {
        self.edits_within(source, &[source.module_range()])
    }

    /// The edits of [`edits`](Self::edits) that fall inside one of
    /// `windows`, ascending and disjoint, walking only the statements
    /// and docstrings a window reaches. Over the module range this is
    /// every edit; over a splice's windows it is the entries a reparse
    /// there could have changed.
    pub(crate) fn edits_within(self, source: &Source, windows: &[TextRange]) -> Vec<Edit> {
        if !self.enabled {
            return Vec::new();
        }
        let mut emitter = Emitter {
            edits: Vec::new(),
            rule: self.rule,
            source,
        };
        emitter.walk_within(source, windows);
        emitter.edits.extend(windows.iter().flat_map(|window| {
            delimiter_padding_gaps(source, *window)
                .filter(|gap| !aligner::is_held(source, self.rule, gap.start()))
                .map(Edit::range_deletion)
        }));
        emitter.edits.retain(|edit| covers(edit.range(), windows));
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

    #[rstest]
    #[case::held_row(
        "x = call( 1 )  # prose: skip[strip-stranded-padding]\ny = [ 2 ]\n",
        true,
        &["[ 2", "2 ]"]
    )]
    #[case::rule_off("x = call( 1 )\n", false, &[])]
    fn edits_skip_a_held_row_and_a_rule_that_is_off(
        #[case] src: &str,
        #[case] enabled: bool,
        #[case] padded: &[&str],
    ) {
        let source = parse(src);
        let stranding = Stranding::new(RuleId::from("strip-stranded-padding"), enabled);
        let gaps: Vec<u32> = stranding
            .edits(&source)
            .iter()
            .map(|edit| edit.start().to_u32())
            .collect();
        let expected: Vec<u32> = padded
            .iter()
            .map(|pair| {
                let at = src.find(pair).expect("the pair is in the source");
                u32::try_from(at + pair.find(' ').expect("a padded pair")).expect("fits")
            })
            .collect();
        assert_eq!(gaps, expected);
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
