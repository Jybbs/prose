//! The row and column measures a `Source` answers: the columns an
//! offset lands at, where its physical and logical lines open and
//! close, and the display width of a span.

use ruff_python_ast::token::TokenKind;
use ruff_python_trivia::lines_before;
use ruff_source_file::{
    LineColumn, LineEnding, LineRanges, OneIndexed, PositionEncoding, SourceLocation,
};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::primitives::{
    comments::trailing_comment,
    inline::{display_width, indent_width},
};

use super::Source;

impl Source {
    /// Returns the zero-indexed character column of `offset` on its line.
    pub fn column_of(&self, offset: TextSize) -> usize {
        self.line_column(offset).column.to_zero_indexed()
    }

    /// Returns `true` when content of display `width` beginning at
    /// `offset`'s column extends past `budget`.
    pub fn column_overflows(&self, offset: TextSize, width: usize, budget: usize) -> bool {
        self.column_of(offset) + width > budget
    }

    /// Returns `true` when `next_start` sits on the source line directly
    /// after `prev_end`'s line. A trailing comment on `prev_end`'s line
    /// keeps the two consecutive, whereas a standalone comment line or a
    /// blank line pushes `next_start` two or more lines down and breaks
    /// adjacency.
    pub fn consecutive_lines(&self, prev_end: TextSize, next_start: TextSize) -> bool {
        self.line_index(next_start) == self.line_index(prev_end).saturating_add(1)
    }

    /// Returns `true` when the source text in `ranged` carries at
    /// least one line break.
    pub fn contains_line_break<R: Ranged>(&self, ranged: R) -> bool {
        self.file.source_text().contains_line_break(ranged.range())
    }

    /// Returns `true` when the physical row holding `offset` continues
    /// the row above it through a `\` explicit line join. A trailing
    /// backslash inside a comment closes with its line and joins
    /// nothing, so it reads as no join.
    pub(crate) fn continues_a_logical_line(&self, offset: TextSize) -> bool {
        let text = self.text();
        let above = &text[..text.line_start(offset).to_usize()];
        let Some(joined) = above
            .strip_suffix('\n')
            .map(|row| row.strip_suffix('\r').unwrap_or(row))
            .or_else(|| above.strip_suffix('\r'))
        else {
            return false;
        };
        let Some(ahead) = joined.strip_suffix('\\') else {
            return false;
        };
        !self.intersects_comment(TextRange::new(TextSize::of(ahead), TextSize::of(joined)))
    }

    /// Returns `true` when at least one blank line separates the
    /// source ahead of `offset` from the preceding non-whitespace.
    pub fn has_blank_line_before(&self, offset: TextSize) -> bool {
        lines_before(offset, self.text()) >= 2
    }

    /// Returns the line and column for a byte offset. Columns count
    /// UTF scalar values (characters), not bytes. Line and column are
    /// both `OneIndexed`.
    pub fn line_column(&self, offset: TextSize) -> LineColumn {
        self.file.to_source_code().line_column(offset)
    }

    /// Returns the line ending this source uses, the first one it
    /// carries, or `LineEnding::Lf` when it carries none.
    pub(crate) fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Returns the character-width of the leading-whitespace prefix on
    /// the line containing `offset`. Tabs and form-feeds count as one
    /// character each.
    pub fn line_indent_width(&self, offset: TextSize) -> usize {
        indent_width(self.text().line_str(offset))
    }

    /// Returns the one-indexed line number for `offset`.
    pub fn line_index(&self, offset: TextSize) -> OneIndexed {
        self.file.to_source_code().line_index(offset)
    }

    /// Returns the range from the start of `offset`'s logical line to
    /// `offset`, the text already placed ahead of it on that line. A
    /// break inside a bracketed construct carries `NonLogicalNewline`,
    /// so the range covers the whole statement rather than one row.
    ///
    /// # Panics
    ///
    /// Panics if `offset` falls inside a token rather than on a
    /// boundary between two.
    pub fn logical_line_start(&self, offset: TextSize) -> TextRange {
        let start = self
            .tokens()
            .before(offset)
            .iter()
            .rev()
            .find(|token| token.kind() == TokenKind::Newline)
            .map_or_else(TextSize::default, Ranged::end);
        TextRange::new(start, offset)
    }

    /// Returns the range from `offset` to the end of its logical line,
    /// the start of the first `Newline` token past it or the module's
    /// own end. A break inside a bracketed construct carries
    /// `NonLogicalNewline` and leaves the logical line open.
    pub fn logical_line_tail(&self, offset: TextSize) -> TextRange {
        let end = self
            .tokens()
            .after(offset)
            .iter()
            .find(|token| token.kind() == TokenKind::Newline)
            .map_or_else(|| self.text().text_len(), Ranged::start);
        TextRange::new(offset, end)
    }

    /// Returns the line-ending sequence used in this source, or
    /// `"\n"` when the source carries no line break.
    pub fn newline_str(&self) -> &'static str {
        self.line_ending().as_str()
    }

    /// Returns the range from `offset` to the end of its physical row,
    /// the columns a construct ending at `offset` shares its row with
    /// once it joins. A construct inside brackets leaves its logical
    /// line open past that row, so [`logical_line_tail`](Self::logical_line_tail)
    /// would charge every row beneath it.
    pub fn row_tail(&self, offset: TextSize) -> TextRange {
        TextRange::new(offset, self.text().line_end(offset))
    }

    /// The display width of the code from `offset` to the end of its
    /// physical row, the columns a construct ending there shares its row
    /// with once it joins. A trailing comment closes the measure, since
    /// charging one against the code budget would let a comment reshape
    /// the code it annotates.
    pub fn row_tail_width(&self, offset: TextSize) -> usize {
        self.tail_width(self.row_tail(offset))
    }

    /// Returns `true` when `a` and `b` sit on one physical source line,
    /// meaning no line break falls in the gap between them.
    pub fn same_line(&self, a: TextSize, b: TextSize) -> bool {
        !self.contains_line_break(TextRange::new(a, b))
    }

    /// Returns the line and character offset for a byte offset, with the
    /// character offset counted in `encoding`'s units. Both line and
    /// character offset are `OneIndexed`. The editor protocol publishes
    /// positions in a negotiated encoding, where `line_column` only ever
    /// counts characters.
    pub fn source_location(&self, offset: TextSize, encoding: PositionEncoding) -> SourceLocation {
        self.file.to_source_code().source_location(offset, encoding)
    }

    /// The display width of the code across `tail`, a span inside one
    /// physical row, closed at a trailing comment the span reaches the
    /// same way [`row_tail_width`](Self::row_tail_width) closes the
    /// whole row.
    pub(crate) fn tail_width(&self, tail: TextRange) -> usize {
        let end = trailing_comment(self, tail.start())
            .map(TextRange::start)
            .filter(|start| tail.contains(*start))
            .unwrap_or(tail.end());
        display_width(self.slice(TextRange::new(tail.start(), end)).trim_end())
    }

    /// Returns the display width of the source text between `a` and `b`.
    pub(crate) fn width_between(&self, a: TextSize, b: TextSize) -> usize {
        display_width(self.slice(TextRange::new(a, b)))
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use ruff_source_file::OneIndexed;

    use super::*;
    use crate::testing::parse;

    fn line_column(line: usize, column: usize) -> LineColumn {
        LineColumn {
            line: OneIndexed::from_zero_indexed(line),
            column: OneIndexed::from_zero_indexed(column),
        }
    }

    #[rstest]
    #[case("a = 1\nb = 2\n", true)]
    #[case("a = 1  # trailing\nb = 2\n", true)]
    #[case("a = 1\n\nb = 2\n", false)]
    #[case("a = 1\n# standalone\nb = 2\n", false)]
    fn consecutive_lines_tolerates_trailing_comment_but_breaks_on_gap(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(
            source.consecutive_lines(body[0].end(), body[1].start()),
            expected,
        );
    }

    #[rstest]
    #[case::joined("\\\ny = 2\n", true)]
    #[case::joined_across_crlf("\\\r\ny = 2\r\n", true)]
    #[case::joined_across_cr("\\\ry = 2\r", true)]
    #[case::plain_row("x = 1\ny = 2\n", false)]
    #[case::opening_row("y = 2\n", false)]
    #[case::comment_head("# leads\ny = 2\n", false)]
    #[case::backslash_closing_a_comment("# trails \\\ny = 2\n", false)]
    fn continues_a_logical_line_reads_the_join_and_not_a_bare_backslash(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let last = source
            .ast()
            .body
            .last()
            .expect("the source carries a statement");

        assert_eq!(source.continues_a_logical_line(last.start()), expected);
    }

    #[test]
    fn line_column_counts_characters_not_bytes() {
        let src = "αβγ";
        let s = parse(src);
        assert_eq!(s.line_column(TextSize::new(6)), line_column(0, 3));
    }

    #[rstest]
    #[case::lf("a\nb\nc\n", &[(0, 0), (2, 1), (4, 2)])]
    #[case::crlf("a\r\nb\r\nc\r\n", &[(0, 0), (3, 1), (6, 2)])]
    #[case::cr("a\rb\rc\r", &[(0, 0), (2, 1), (4, 2)])]
    fn line_column_reads_every_line_ending(#[case] src: &str, #[case] rows: &[(u32, usize)]) {
        let s = parse(src);
        for &(offset, line) in rows {
            assert_eq!(s.line_column(TextSize::new(offset)), line_column(line, 0));
        }
    }

    #[rstest]
    #[case("a\nb\n", LineEnding::Lf)]
    #[case("a\r\nb\r\n", LineEnding::CrLf)]
    #[case("a\rb\r", LineEnding::Cr)]
    #[case("a\nb\r\n", LineEnding::Lf)]
    #[case("a\r\nb\n", LineEnding::CrLf)]
    #[case("x = 1", LineEnding::Lf)]
    fn line_ending_reads_the_first_break_and_falls_back_to_lf(
        #[case] src: &str,
        #[case] expected: LineEnding,
    ) {
        assert_eq!(parse(src).line_ending(), expected);
        assert_eq!(parse(src).newline_str(), expected.as_str());
    }

    #[rstest]
    #[case("a = 1; b = 2\n", true)]
    #[case("a = 1\nb = 2\n", false)]
    fn same_line_holds_within_a_line_and_breaks_across_one(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let body = &source.ast().body;
        assert_eq!(source.same_line(body[0].end(), body[1].start()), expected);
    }
}
