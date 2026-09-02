//! The token and bracket reads a `Source` answers: the parenthesized
//! range an expression recovers against its parent, the tokens a span
//! overlaps, and the literals and replacement fields a walk seeds from.

use itertools::Itertools;
use ruff_python_ast::{
    AnyNodeRef, ExprRef,
    token::{Token, TokenKind, parenthesized_range},
};
use ruff_python_trivia::{BackwardsTokenizer, SimpleToken, SimpleTokenKind};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::primitives::{
    layout::{is_layoutable, requires_expand},
    tokens::is_interpolated_string_start,
    walk::{Descent, filter_map_over_exprs},
};

use super::Source;

impl Source {
    /// The count of f-strings and t-strings open at `offset`, read off
    /// the ascending spans of every opener and closer pair, built on the
    /// first read.
    pub(crate) fn interpolation_depth_at(&self, offset: TextSize) -> usize {
        let spans = self.interpolation_spans.get_or_init(|| {
            let mut open = Vec::new();
            let mut spans = Vec::new();
            for token in self.tokens().iter() {
                if is_interpolated_string_start(token.kind()) {
                    open.push(token.start());
                } else if token.kind().is_interpolated_string_end()
                    && let Some(start) = open.pop()
                {
                    spans.push(TextRange::new(start, token.end()));
                }
            }
            spans.sort_unstable_by_key(Ranged::start);
            spans
        });
        spans
            .iter()
            .take_while(|span| span.start() <= offset)
            .filter(|span| span.contains(offset))
            .count()
    }

    /// The start offsets of the non-trivia tokens an `(` directly
    /// precedes, built on the first read.
    fn paren_followers(&self) -> &FxHashSet<TextSize> {
        self.paren_followers.get_or_init(|| {
            self.tokens()
                .iter()
                .filter(|token| !token.kind().is_trivia())
                .tuple_windows()
                .filter(|(open, _)| open.kind() == TokenKind::Lpar)
                .map(|(_, follower)| follower.start())
                .collect()
        })
    }

    /// Returns the first non-trivia token scanning backward from
    /// `offset`, or `None` when the scan finds none.
    fn prev_non_trivia_token(&self, offset: TextSize) -> Option<SimpleToken> {
        BackwardsTokenizer::up_to(offset, self.text(), self.comment_ranges())
            .skip_trivia()
            .next()
    }

    /// Returns the start-ascending ranges of the comment-free literals
    /// `reflow-collections` can expand, walking the tree on the first
    /// read.
    pub(crate) fn expandable_literals(&self) -> &[TextRange] {
        self.expandable_literals.get_or_init(|| {
            filter_map_over_exprs(&self.ast().body, Descent::Over, |expr| {
                (is_layoutable(expr)
                    && requires_expand(expr)
                    && !self.intersects_comment(expr.range()))
                .then_some(expr.range())
            })
        })
    }

    /// Returns the start offset of the first token in `range` for
    /// which `predicate` is true. Callers that need the full `&Token`
    /// (kind, range, flags) should chain
    /// `tokens().in_range(range).iter().find(...)` directly.
    pub fn first_token_offset_in_range<F>(
        &self,
        range: TextRange,
        mut predicate: F,
    ) -> Option<TextSize>
    where
        F: FnMut(&Token) -> bool,
    {
        self.tokens()
            .in_range(range)
            .iter()
            .find(|&t| predicate(t))
            .map(Token::start)
    }

    /// Returns `expr`'s range widened to the explicit parentheses
    /// recovered against `parent`, falling back to the bare expression
    /// range when none enclose it.
    pub(crate) fn paren_aware_range(&self, expr: ExprRef, parent: AnyNodeRef) -> TextRange {
        self.parenthesized_range(expr, parent)
            .unwrap_or_else(|| expr.range())
    }

    /// The range of `expr` including the parentheses recovered against
    /// `parent`, `None` where none enclose it.
    pub(crate) fn parenthesized_range(
        &self,
        expr: ExprRef,
        parent: AnyNodeRef,
    ) -> Option<TextRange> {
        self.paren_followers()
            .contains(&expr.start())
            .then(|| parenthesized_range(expr, parent, self.tokens()))
            .flatten()
    }

    /// Returns the end offset of the token preceding `offset`, scanning
    /// backward over whitespace and comments.
    pub(crate) fn prev_token_end(&self, offset: TextSize) -> TextSize {
        self.prev_non_trivia_token(offset)
            .expect("invariant: a token precedes the scanned offset")
            .end()
    }

    /// `expr`'s range, widened to its recovered parentheses only where
    /// its own text spans rows, the pair holding those rows together
    /// once the text around it joins.
    pub(crate) fn spanning_paren_range(&self, expr: ExprRef, parent: AnyNodeRef) -> TextRange {
        if self.contains_line_break(expr.range()) {
            self.paren_aware_range(expr, parent)
        } else {
            expr.range()
        }
    }

    /// Yields each adjacent token pair with the source range between
    /// them, the trivia the lexer skipped.
    pub(crate) fn token_gaps(&self) -> impl Iterator<Item = (&Token, &Token, TextRange)> {
        self.tokens()
            .iter()
            .tuple_windows()
            .map(|(token, next)| (token, next, TextRange::new(token.end(), next.start())))
    }

    /// Yields the tokens overlapping `range`, opening at the nearest
    /// token start at or before `range.start()`, so a boundary inside a
    /// token still reaches the token spanning it.
    pub(crate) fn tokens_overlapping(&self, range: TextRange) -> impl Iterator<Item = &Token> {
        let tokens = self.tokens();
        let first = tokens
            .binary_search_by_start(range.start())
            .unwrap_or_else(|slot| slot.saturating_sub(1));
        tokens[first..]
            .iter()
            .take_while(move |token| token.start() < range.end())
    }

    /// Returns the range of the trailing comma immediately before the
    /// closing bracket of `container`, or `None` when the last
    /// non-trivia token there is not a comma.
    pub(crate) fn trailing_comma(&self, container: TextRange) -> Option<TextRange> {
        self.prev_non_trivia_token(container.end() - TextSize::from(1u32))
            .filter(|token| token.kind() == SimpleTokenKind::Comma)
            .map(|token| token.range)
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;
    use ruff_python_ast::token::TokenKind;

    use ruff_text_size::TextRange;

    use super::*;
    use crate::{
        primitives::{scope::sub_bodies, walk::filter_map_over_parented_exprs},
        testing::parse,
    };

    #[rstest]
    #[case::the_first_of_two_matches("a = b = 1\n", |t: &Token| t.kind() == TokenKind::Equal, Some(2))]
    #[case::a_single_match("x = 1\n", |t: &Token| t.kind() == TokenKind::Equal, Some(2))]
    #[case::no_match("x = 1\n", |t: &Token| t.kind() == TokenKind::Colon, None)]
    #[case::a_predicate_family("x += 1\n", |t: &Token| t.kind().as_augmented_assign_operator().is_some(), Some(2))]
    fn first_token_offset_in_range_returns_the_leftmost_match(
        #[case] src: &str,
        #[case] predicate: fn(&Token) -> bool,
        #[case] expected: Option<u32>,
    ) {
        let s = parse(src);
        let found = s.first_token_offset_in_range(s.ast().body[0].range(), predicate);
        assert_eq!(found, expected.map(TextSize::new));
    }

    #[test]
    fn first_token_offset_in_range_returns_none_for_empty_range() {
        let s = parse("x = 1\n");
        let empty = TextRange::empty(TextSize::new(0));

        assert!(s.first_token_offset_in_range(empty, |_| true).is_none());
    }

    #[rstest]
    #[case("f(a)\n")]
    #[case("(a)\n")]
    #[case("((a))\n")]
    #[case("x = (  # c\n    a\n)\n")]
    #[case("[a]\n")]
    #[case("f((a), (b))\n")]
    #[case("x = a if (b) else c\n")]
    fn parenthesized_range_agrees_with_the_token_walk(#[case] src: &str) {
        let source = parse(src);
        let pairs = filter_map_over_parented_exprs(source.ast(), Descent::Into, |expr, parent| {
            Some((expr, parent))
        });
        assert!(!pairs.is_empty());
        for (expr, parent) in pairs {
            assert_eq!(
                source.parenthesized_range(expr.into(), parent),
                parenthesized_range(expr.into(), parent, source.tokens()),
                "{src:?} at {:?}",
                expr.range()
            );
        }
    }

    #[rstest]
    #[case("class C:\n    pass\n", "class C:")]
    #[case("class C:  # eol\n    pass\n", "class C:")]
    #[case("class C:\n    # comment\n    pass\n", "class C:")]
    #[case("def f():\n    pass\n", "def f():")]
    #[case("def f(\n    x,\n    y,\n):\n    pass\n", "def f(\n    x,\n    y,\n):")]
    fn prev_token_end_lands_past_the_header_colon(#[case] src: &str, #[case] header: &str) {
        let source = parse(src);
        let (body, _) = sub_bodies(&source.ast().body[0])[0];
        let end = source.prev_token_end(body[0].start());
        assert_eq!(&source.text()[..end.to_usize()], header);
    }
}
