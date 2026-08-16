//! Sheds a backslash line continuation, the escape that splits a
//! statement on a physical newline rather than inside a bracket. A
//! continuation a bracket already spans drops the backslash and keeps
//! its break, leaving the bracket's shape to the layout rules. One
//! outside every bracket rejoins onto a single line where the joined
//! line fits the budget, and otherwise parenthesizes the outermost
//! expression spanning the break so the bracketed form carries the
//! split. A backslash the lexer folded into a continued indentation is
//! left alone, since no join preserves the indent it declares.

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, find_node::covering_node, token::TokenKind};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        edit::{apply_inline_edits, narrowed_replacement},
        range::blocks_span,
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ShedBackslashContinuations {
    code_line_length: usize,
}

impl ShedBackslashContinuations {
    pub(crate) const MESSAGE: &'static str = "shed a backslash line continuation";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
        }
    }

    /// The fix for one run of unbracketed gaps, the join where the
    /// merged line fits the budget, and otherwise the parenthesized
    /// break wherever an expression spans the run.
    fn shed_run(&self, source: &Source, run: &[Gap]) -> Vec<Edit> {
        let span = blocks_span(run);
        let joined = join_edits(source, run);
        if joined_width(source, span, &joined) <= self.code_line_length {
            return joined;
        }
        wrap_edits(source, span, run).unwrap_or(joined)
    }
}

impl Rule for ShedBackslashContinuations {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let gaps = continuation_gaps(source);
        gaps.chunk_by(|earlier, later| shares_a_run(source, earlier, later))
            .filter_map(|run| match run {
                [gap] if gap.bracketed => stripped_edit(source, gap.range).map(|edit| vec![edit]),
                _ => Some(self.shed_run(source, run)),
            })
            .collect()
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// One inter-token gap carrying at least one backslash continuation.
struct Gap {
    /// True when a bracket stays open across the gap, so the break
    /// needs no backslash and survives the shed.
    bracketed: bool,
    /// The text a join leaves between the gap's two tokens, one space
    /// where the pair takes a separator and empty where it abuts, as a
    /// chain's `.`, a call's `(`, and the newline closing a logical
    /// line all do.
    join: &'static str,
    range: TextRange,
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
fn continuation_gaps(source: &Source) -> Vec<Gap> {
    let mut depth = 0usize;
    let mut gaps = Vec::new();
    for (token, next, range) in source.token_gaps() {
        let kind = token.kind();
        if is_opener(kind) {
            depth += 1;
        } else if is_closer(kind) {
            depth = depth.saturating_sub(1);
        }
        if !source.slice(range).contains('\\') {
            continue;
        }
        if depth == 0 && source.text().is_at_start_of_line(range.start()) {
            continue;
        }
        gaps.push(Gap {
            bracketed: depth > 0,
            join: join_text(kind, next.kind()),
            range,
        });
    }
    gaps
}

/// True for a token closing an atom, the left side a call's `(` or a
/// subscript's `[` binds against.
fn ends_atom(kind: TokenKind) -> bool {
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

/// Replaces each gap in `run` with its join text, folding the physical
/// lines the run continues onto one.
fn join_edits(source: &Source, run: &[Gap]) -> Vec<Edit> {
    run.iter()
        .filter_map(|gap| narrowed_replacement(source, gap.range, gap.join.to_owned()))
        .collect()
}

/// The text a join leaves between `token` and `next`, empty where the
/// pair abuts with no space and one space otherwise.
fn join_text(token: TokenKind, next: TokenKind) -> &'static str {
    let abuts = token == TokenKind::Dot
        || next.is_any_newline()
        || matches!(
            next,
            TokenKind::Colon | TokenKind::Comma | TokenKind::Dot | TokenKind::Semi
        )
        || (matches!(next, TokenKind::Lpar | TokenKind::Lsqb) && ends_atom(token));
    if abuts { "" } else { " " }
}

/// The display width of the physical line `edits` fold `span` onto,
/// measured from the opening line's first column through the closing
/// line's last.
fn joined_width(source: &Source, span: TextRange, edits: &[Edit]) -> usize {
    apply_inline_edits(source, source.text().lines_range(span), edits).width()
}

/// True when `earlier` and `later` are both unbracketed and their joins
/// land on one physical line, the test gathering consecutive gaps into
/// a run.
fn shares_a_run(source: &Source, earlier: &Gap, later: &Gap) -> bool {
    !earlier.bracketed && !later.bracketed && source.same_line(earlier.end(), later.start())
}

/// Drops every backslash in `gap` along with the whitespace ahead of
/// it, keeping the line breaks the gap carries. Returns `None` where
/// the gap already reads that way.
fn stripped_edit(source: &Source, gap: TextRange) -> Option<Edit> {
    narrowed_replacement(source, gap, stripped_gap(source, gap))
}

/// `gap`'s text with each line-ending backslash and the whitespace
/// ahead of it removed, dropping a physical line the backslash leaves
/// empty and keeping the closing line's indentation.
fn stripped_gap(source: &Source, gap: TextRange) -> String {
    let opens_line = source.text().is_at_start_of_line(gap.start());
    source
        .slice(gap)
        .split('\n')
        .map(|segment| segment.strip_suffix('\r').unwrap_or(segment))
        .with_position()
        .filter_map(|(position, segment)| {
            let stripped = segment.strip_suffix('\\').map_or(segment, str::trim_end);
            let strands_a_line = stripped.is_empty() && (!position.is_first || opens_line);
            (position.is_last || !strands_a_line).then_some(stripped)
        })
        .join(source.newline_str())
}

/// Parenthesizes the outermost expression spanning `run` and drops the
/// run's backslashes, keeping every break. Returns `None` where no
/// expression spans the run or the wrapped form reparses to a different
/// tree.
fn wrap_edits(source: &Source, span: TextRange, run: &[Gap]) -> Option<Vec<Edit>> {
    let root = AnyNodeRef::from(source.ast());
    let wrapped = covering_node(root, span)
        .find_last(AnyNodeRef::is_expression)
        .ok()?
        .node()
        .range();
    let mut edits: Vec<Edit> = run
        .iter()
        .filter_map(|gap| stripped_edit(source, gap.range))
        .collect();
    let candidate = format!("({})", apply_inline_edits(source, wrapped, &edits));
    splice_preserves_tree(source, wrapped, &candidate).then(|| {
        edits.insert(0, Edit::insertion("(".to_owned(), wrapped.start()));
        edits.push(Edit::insertion(")".to_owned(), wrapped.end()));
        edits
    })
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
    #[case(TokenKind::Plus, TokenKind::Int, " ")]
    #[case(TokenKind::Equal, TokenKind::Lsqb, " ")]
    #[case(TokenKind::Name, TokenKind::Dot, "")]
    #[case(TokenKind::Dot, TokenKind::Name, "")]
    #[case(TokenKind::Name, TokenKind::Lsqb, "")]
    #[case(TokenKind::Name, TokenKind::Lpar, "")]
    #[case(TokenKind::Name, TokenKind::Comma, "")]
    #[case(TokenKind::Int, TokenKind::Newline, "")]
    fn join_text_spaces_only_where_the_pair_takes_one(
        #[case] token: TokenKind,
        #[case] next: TokenKind,
        #[case] expected: &str,
    ) {
        assert_eq!(join_text(token, next), expected);
    }

    #[rstest]
    #[case("x = 1 + \\\n    2\n", "x = 1 + 2")]
    #[case("x = 1 + \\\n    2  # note\n", "x = 1 + 2  # note")]
    fn joined_width_measures_the_line_the_run_produces(#[case] src: &str, #[case] expected: &str) {
        let source = parse(src);
        let gaps = continuation_gaps(&source);
        let edits = join_edits(&source, &gaps);
        assert_eq!(
            joined_width(&source, blocks_span(&gaps), &edits),
            expected.len(),
        );
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

    #[test]
    fn wrap_edits_declines_a_break_no_expression_spans() {
        let source = parse("import alpha, \\\n    beta\n");
        let gaps = continuation_gaps(&source);
        assert!(wrap_edits(&source, blocks_span(&gaps), &gaps).is_none());
    }
}
