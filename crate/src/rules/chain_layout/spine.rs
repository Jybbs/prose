//! The receiver and call links a fluent chain divides into, each range
//! stopping at its last non-whitespace character so the text between two
//! segments is the gap a break rewrites.

use ruff_python_ast::{Expr, ExprAttribute, token::TokenKind};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::source::Source;

/// A chain's receiver and its `.name(...)` links in source order, each
/// link running from its own dot to the one opening the link below it,
/// or to the chain's end for the last.
pub(super) struct Chain {
    pub(super) links: Vec<TextRange>,
    pub(super) receiver_range: TextRange,
}

impl Chain {
    /// The chain `expr` divides into, or `None` when its spine carries
    /// fewer than two links. A `.name` access that is not itself called
    /// opens no link of its own, moving the opening of the link below it
    /// down onto its own dot, and a trailing one stays with the link
    /// above it.
    pub(super) fn of(source: &Source, expr: &Expr) -> Option<Self> {
        let mut dots: Vec<TextSize> = Vec::new();
        let mut cursor = expr;
        loop {
            let attribute = match cursor {
                Expr::Attribute(attribute) => {
                    if let Some(dot) = dots.last_mut() {
                        *dot = dot_offset(source, attribute);
                    }
                    attribute
                }
                Expr::Call(call) => match call.func.as_ref() {
                    Expr::Attribute(attribute) => {
                        dots.push(dot_offset(source, attribute));
                        attribute
                    }
                    _ => break,
                },
                _ => break,
            };
            cursor = attribute.value.as_ref();
        }
        if dots.len() < 2 {
            return None;
        }
        dots.reverse();
        let stops = dots[1..].iter().copied().chain([expr.end()]);
        Some(Self {
            links: dots
                .iter()
                .zip(stops)
                .map(|(&dot, stop)| trimmed(source, TextRange::new(dot, stop)))
                .collect(),
            receiver_range: trimmed(source, TextRange::new(expr.start(), dots[0])),
        })
    }

    /// Every segment in source order, the receiver then each link.
    fn segments(&self) -> impl Iterator<Item = TextRange> + '_ {
        std::iter::once(self.receiver_range).chain(self.links.iter().copied())
    }

    /// The receiver's display width, the columns a hung link's dot sits
    /// past the indent the broken chain opens at.
    pub(super) fn receiver_width(&self, source: &Source) -> usize {
        source.slice(self.receiver_range).width()
    }

    /// True when a segment carries a line break, the shape a break
    /// declines rather than reassemble onto its rows.
    pub(super) fn spans_lines(&self, source: &Source) -> bool {
        self.segments()
            .any(|segment| source.contains_line_break(segment))
    }

    /// The display width the chain reads joined onto one line.
    pub(super) fn width(&self, source: &Source) -> usize {
        self.segments()
            .map(|segment| source.slice(segment).width())
            .sum()
    }
}

/// The offset of the `.` separating `attribute`'s value from its name.
fn dot_offset(source: &Source, attribute: &ExprAttribute) -> TextSize {
    source
        .first_token_offset_in_range(
            TextRange::new(attribute.value.end(), attribute.attr.start()),
            |token| token.kind() == TokenKind::Dot,
        )
        .expect("an attribute carries a `.` between its value and its name")
}

/// `range` shortened past the whitespace it ends with.
fn trimmed(source: &Source, range: TextRange) -> TextRange {
    let text = source.slice(range);
    range.sub_end(text.text_len() - text.trim_end().text_len())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    /// The chain `source`'s first assigned value divides into.
    fn chain_of(source: &Source) -> Option<Chain> {
        Chain::of(source, first_value(source))
    }

    #[rstest]
    fn chain_of_declines_a_spine_under_two_links(
        #[values(
            "x = base.one()\n",
            "x = settings.rules.chain.cap()\n",
            "x = plain(arg)\n",
            "x = base.one().attr\n"
        )]
        src: &str,
    ) {
        assert!(chain_of(&parse(src)).is_none());
    }

    #[rstest]
    #[case("x = base.one().two()\n", &["base", ".one()", ".two()"][..])]
    #[case("x = base.one().mid.two()\n", &["base", ".one()", ".mid.two()"][..])]
    #[case("x = (a + b).one().two()\n", &["(a + b)", ".one()", ".two()"][..])]
    #[case("x = base[key].one().two()\n", &["base[key]", ".one()", ".two()"][..])]
    #[case("x = base.one() .two()\n", &["base", ".one()", ".two()"][..])]
    #[case("x = base.one().two().attr\n", &["base", ".one()", ".two().attr"][..])]
    fn chain_of_divides_a_spine_into_its_receiver_and_links(
        #[case] src: &str,
        #[case] expected: &[&str],
    ) {
        let source = parse(src);
        let chain = chain_of(&source).expect("the spine carries two links");
        let segments: Vec<&str> = chain.segments().map(|s| source.slice(s)).collect();
        assert_eq!(segments, expected);
    }

    #[rstest]
    #[case("x = base.one().two()\n", false)]
    #[case("x = (\n    base.one()\n        .two()\n)\n", false)]
    #[case("x = base.one(\n    arg,\n).two()\n", true)]
    fn spans_lines_reads_the_segments_rather_than_the_gaps(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let chain = chain_of(&source).expect("the spine carries two links");
        assert_eq!(chain.spans_lines(&source), expected);
    }

    #[test]
    fn width_measures_the_joined_form_past_the_source_spacing() {
        let source = parse("x = base.one() .two()\n");
        let chain = chain_of(&source).expect("the spine carries two links");
        assert_eq!(chain.width(&source), "base.one().two()".len());
    }
}
