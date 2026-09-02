//! The backslash-joined gaps of a logical line and the runs they form.

use itertools::Itertools;
use ruff_python_ast::token::TokenKind;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use super::*;

/// One inter-token gap carrying at least one backslash continuation.
pub(super) struct Gap {
    /// True when a bracket stays open across the gap, so the break
    /// needs no backslash and survives the shed.
    pub(super) bracketed: bool,
    /// The text a join leaves between the gap's two tokens, one space
    /// where the pair takes a separator and empty where it abuts, as a
    /// chain's `.`, a call's `(`, and the newline closing a logical
    /// line all do.
    pub(super) join: &'static str,
    pub(super) range: TextRange,
}

impl Ranged for Gap {
    fn range(&self) -> TextRange {
        self.range
    }
}

/// Every inter-token gap carrying a backslash continuation, in source
/// order. A gap opening a physical line outside every bracket is
/// skipped, its backslash sitting where the join would rewrite the
/// indentation.
pub(super) fn continuation_gaps(source: &Source) -> Vec<Gap> {
    let mut depth = 0usize;
    source
        .token_gaps()
        .filter_map(|(token, next, range)| {
            let kind = token.kind();
            if is_opener(kind) {
                depth += 1;
            } else if is_closer(kind) {
                depth = depth.saturating_sub(1);
            }
            let held = !source.slice(range).contains('\\')
                || (depth == 0 && source.text().is_at_start_of_line(range.start()));
            (!held).then(|| Gap {
                bracketed: depth > 0,
                join: join_text(kind, next.kind()),
                range,
            })
        })
        .collect()
}

/// True for a token closing an atom, the left side a call's `(` or a
/// subscript's `[` binds against.
pub(super) fn ends_atom(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Complex
            | TokenKind::Ellipsis
            | TokenKind::False
            | TokenKind::Float
            | TokenKind::FStringEnd
            | TokenKind::Int
            | TokenKind::Name
            | TokenKind::None
            | TokenKind::Rbrace
            | TokenKind::Rpar
            | TokenKind::Rsqb
            | TokenKind::String
            | TokenKind::TStringEnd
            | TokenKind::True
    )
}

/// True when `earlier` and `later` are both unbracketed and their joins
/// land on one physical line, the test gathering consecutive gaps into
/// a run.
pub(super) fn shares_a_run(source: &Source, earlier: &Gap, later: &Gap) -> bool {
    !earlier.bracketed && !later.bracketed && source.same_line(earlier.end(), later.start())
}

/// `gap`'s text with each line-ending backslash and the whitespace
/// ahead of it removed, dropping a physical line the backslash leaves
/// empty and keeping the closing line's indentation.
pub(super) fn stripped_gap(source: &Source, gap: TextRange) -> String {
    let opens_line = source.text().is_at_start_of_line(gap.start());
    source
        .slice(gap)
        .split(['\r', '\n'])
        .with_position()
        .filter_map(|(position, segment)| {
            let stripped = segment.strip_suffix('\\').map_or(segment, str::trim_end);
            let strands_a_line = stripped.is_empty() && (!position.is_first || opens_line);
            (position.is_last || !strands_a_line).then_some(stripped)
        })
        .join(source.newline_str())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn continuation_gaps_flags_a_bracketed_break() {
        let gaps = continuation_gaps(&parse("x = (1 + \\\n    2)\n"));
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].bracketed);
    }

    #[test]
    fn continuation_gaps_skips_a_backslash_the_indent_swallows() {
        assert!(continuation_gaps(&parse("if ready:\n   \\\n    started = True\n")).is_empty());
    }

    #[rstest]
    #[case(TokenKind::Name, true)]
    #[case(TokenKind::Rsqb, true)]
    #[case(TokenKind::String, true)]
    #[case(TokenKind::Equal, false)]
    #[case(TokenKind::Plus, false)]
    fn ends_atom_flags_the_tokens_a_call_or_subscript_binds_against(
        #[case] kind: TokenKind,
        #[case] expected: bool,
    ) {
        assert_eq!(ends_atom(kind), expected);
    }

    #[rstest]
    #[case("x = 1 + \\\n    2 + \\\n    3\n", &[2])]
    #[case("x = 1 + \\\n    2\ny = 3 + \\\n    4\n", &[1, 1])]
    #[case("x = (1 + \\\n    2)\ny = 1 + \\\n    2\n", &[1, 1])]
    fn gaps_chunk_into_runs_landing_on_one_line(#[case] src: &str, #[case] expected: &[usize]) {
        let source = parse(src);
        let gaps = continuation_gaps(&source);
        let lengths: Vec<usize> = gaps
            .chunk_by(|earlier, later| shares_a_run(&source, earlier, later))
            .map(|run| run.len())
            .collect();
        assert_eq!(lengths, expected);
    }

    #[rstest]
    #[case("x = (1 + \\\n    2)\n", "\n    ")]
    #[case("x = (1 +\n     \\\n     2)\n", "     ")]
    fn stripped_gap_keeps_the_break_and_drops_a_stranded_line(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let gaps = continuation_gaps(&source);
        assert_eq!(stripped_gap(&source, gaps[0].range), expected);
    }

    #[test]
    fn stripped_gap_rejoins_with_the_source_line_ending() {
        let source = parse("x = (1 + \\\r\n    2)\r\n");
        let gaps = continuation_gaps(&source);
        assert_eq!(stripped_gap(&source, gaps[0].range), "\r\n    ");
    }
}
