//! Strips padding that aligns with nothing. On a colon context with no
//! column to align to it clears the pre-colon gap and collapses the
//! post-colon gap to one space, and it clears the space just inside a
//! bracket delimiter. Runs after the alignment rules in
//! `Pipeline::with_defaults` so it sees their output.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::TokenKind;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        aligner,
        colon_targets::{ColonEmitter, ColonMember},
        edit::singleton_groups,
    },
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
        emitter.edits.extend(delimiter_padding_edits(source));
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
    fn handle(&mut self, members: &[ColonMember]) {
        let aligned: Vec<aligner::Member> = members.iter().map(|m| m.member).collect();
        if aligner::is_alignment_candidate(self.source, &aligned) {
            return;
        }
        for m in members {
            if m.member.width > 0 {
                self.edits
                    .extend(aligner::space_padding_edit(self.source, m.member.gap, 0));
            }
            if let Some(gap) = m.single_line_value_gap(self.source) {
                self.edits
                    .extend(aligner::space_padding_edit(self.source, gap, 1));
            }
        }
    }

    fn rule(&self) -> RuleId {
        StripAlignPadding::SLUG
    }
}

/// Deletes the whitespace run directly inside a bracket delimiter,
/// after an opening `(` `[` `{` or before its closer, when the run
/// shares a line with the neighbor it pads against. A closer on its own
/// line keeps its leading indent, since the gap then spans a line
/// break. Tokens inside an f-string or t-string replacement field stay
/// untouched, tracked through `interp_depth`.
fn delimiter_padding_edits(source: &Source) -> Vec<Edit> {
    let tokens = source.tokens();
    let mut interp_depth: u32 = 0;
    let mut edits = Vec::new();
    for (token, next) in tokens.iter().zip(tokens.iter().skip(1)) {
        let kind = token.kind();
        if matches!(kind, TokenKind::FStringStart | TokenKind::TStringStart) {
            interp_depth += 1;
        } else if kind.is_interpolated_string_end() {
            interp_depth -= 1;
        }
        if interp_depth > 0 {
            continue;
        }
        let gap = TextRange::new(token.end(), next.start());
        if gap.is_empty() || source.contains_line_break(gap) {
            continue;
        }
        if (is_opener(kind) && !next.kind().is_trivia())
            || (is_closer(next.kind()) && !kind.is_trivia())
        {
            edits.push(Edit::range_deletion(gap));
        }
    }
    edits
}

/// Returns `true` when `kind` is a closing bracket `)` `]` `}`.
fn is_closer(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace)
}

/// Returns `true` when `kind` is an opening bracket `(` `[` `{`.
fn is_opener(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_text_size::{Ranged, TextSize};

    use super::*;
    use crate::testing::{parse, range};

    fn run_strip(source: &Source, members: &[aligner::Member]) -> Vec<Edit> {
        let colon_members: Vec<ColonMember> = members
            .iter()
            .map(|&member| ColonMember {
                member,
                value_gap: None,
            })
            .collect();
        let mut emitter = Emitter {
            edits: Vec::new(),
            source,
        };
        emitter.handle(&colon_members);
        emitter.edits
    }

    #[test]
    fn delimiter_skips_closer_on_its_own_line() {
        // The closer carries leading indent rather than interior padding
        // once a line break separates it from the content.
        assert!(delimiter_padding_edits(&parse("x = [\n    1\n    ]\n")).is_empty());
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
        assert!(delimiter_padding_edits(&parse(src)).is_empty());
    }

    #[test]
    fn delimiter_skips_padding_before_a_comment() {
        // Padding between an opener and a same-line comment is left, so the
        // comment does not fuse onto the bracket.
        assert!(delimiter_padding_edits(&parse("f(  # note\n    a,\n)\n")).is_empty());
    }

    #[test]
    fn delimiter_strips_after_opener_and_before_closer() {
        let edits = delimiter_padding_edits(&parse("f( 1 )\n"));
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].range(), range(2, 3));
        assert_eq!(edits[1].range(), range(4, 5));
    }

    #[test]
    fn delimiter_strips_empty_pair_once() {
        // Both sides of `f( )` qualify, yet the lone gap emits a single
        // edit rather than an overlapping pair.
        let edits = delimiter_padding_edits(&parse("f( )\n"));
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range(), range(2, 3));
    }

    #[rstest]
    #[case(TokenKind::Rpar, true)]
    #[case(TokenKind::Rsqb, true)]
    #[case(TokenKind::Rbrace, true)]
    #[case(TokenKind::Lpar, false)]
    #[case(TokenKind::Name, false)]
    fn is_closer_flags_closing_brackets(#[case] kind: TokenKind, #[case] expected: bool) {
        assert_eq!(is_closer(kind), expected);
    }

    #[rstest]
    #[case(TokenKind::Lpar, true)]
    #[case(TokenKind::Lsqb, true)]
    #[case(TokenKind::Lbrace, true)]
    #[case(TokenKind::Rpar, false)]
    #[case(TokenKind::Name, false)]
    fn is_opener_flags_opening_brackets(#[case] kind: TokenKind, #[case] expected: bool) {
        assert_eq!(is_opener(kind), expected);
    }

    #[test]
    fn strip_handles_empty_members_slice() {
        assert!(run_strip(&parse(""), &[]).is_empty());
    }

    #[test]
    fn strip_leaves_a_value_gap_that_crosses_a_line_break() {
        // A colon whose value opens on a later line keeps its placement:
        // the pre-colon padding strips, but the post-colon gap is not
        // collapsed across the break.
        let source = parse("d = {\"k\"  :\n    v}\n");
        let member = aligner::Member {
            gap: range(8, 10),
            line_start: TextSize::new(0),
            op_width: 0,
            width: 3,
        };
        let colon_member = ColonMember {
            member,
            value_gap: Some(range(11, 16)),
        };
        let mut emitter = Emitter {
            edits: Vec::new(),
            source: &source,
        };
        emitter.handle(&[colon_member]);
        assert_eq!(emitter.edits.len(), 1);
        assert_eq!(emitter.edits[0].range(), range(8, 10));
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
    fn strip_strips_multi_member_groups_at_differing_baselines() {
        // Distinct lines opening at different indents (free inside the
        // brackets), so the `:`s share no column and the pre-`:` padding
        // strips the way a singleton's does.
        let source = parse("d = {\n    \"ab\"  : 1,\n        \"cd\"  : 2,\n}\n");
        let members = [
            aligner::Member {
                gap: range(14, 16),
                line_start: TextSize::new(6),
                op_width: 0,
                width: 4,
            },
            aligner::Member {
                gap: range(33, 35),
                line_start: TextSize::new(21),
                op_width: 0,
                width: 4,
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
