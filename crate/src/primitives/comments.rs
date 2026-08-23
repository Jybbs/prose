//! Own-line comment-block detection between two statements, covering
//! the leading block, whether it reads as a decorative banner or a
//! multi-hash heading, and where the block binding to the member below
//! it starts. A run anchors in place on a section marker, a suppression
//! directive, or a tool pragma, and binds to the member otherwise,
//! whatever blank line sits between the two. A whole-line deletion of a
//! member strands the run leading it, which the import prune reads
//! before it deletes. The trailing-comment gap the banding and spacing
//! rules both seat lives here as well.

use ruff_python_ast::ExprDict;
use ruff_python_trivia::{CommentRanges, PythonWhitespace, is_pragma_comment};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::{
    primitives::blanks::whitespace_start_before, source::Source, suppression::is_directive_comment,
};

/// The characters whose appearance directly after a comment's hash run
/// leaves the opener untouched, covering the shebang, the quoted and
/// piped forms, and the structured `#:` attribute-doc marker.
const EXEMPT_LEADERS: [char; 4] = ['!', '\'', ':', '|'];

/// The gap PEP 8 seats between code and a trailing comment.
pub(crate) const TRAILING_GAP: &str = "  ";

/// True when `block` holds its position rather than binding to the
/// member below it, carrying a section marker, a suppression directive,
/// or a tool pragma on any of its lines.
pub(crate) fn anchors_in_place(source: &Source, block: TextRange) -> bool {
    source
        .slice(block)
        .lines()
        .map(str::trim_start)
        .any(|line| is_marker_line(line) || is_directive_comment(line) || is_pragma_comment(line))
}

/// The start of the block binding to the member at `item_start`, the
/// own-line comment run in `[lower, item_start)` when that run binds,
/// or `item_start`'s line start when the run anchors in place, opens at
/// another indent, or no comment sits there. A blank line above the
/// member leaves the run bound.
pub(super) fn bound_block_start(
    source: &Source,
    lower: TextSize,
    item_start: TextSize,
) -> TextSize {
    let line_start = source.text().line_start(item_start);
    if lower > line_start {
        return item_start;
    }
    leading_comment_block(source, lower, item_start)
        .filter(|block| {
            !anchors_in_place(source, *block)
                && source.line_indent_width(block.start()) == source.line_indent_width(item_start)
        })
        .map_or(line_start, TextRange::start)
}

/// True when an own-line comment block leads the item at `item_start`,
/// reached across the blank run between the two and stopped at a
/// notebook cell wall. A whole-line deletion of that item strands the
/// block.
pub(super) fn comment_leads(source: &Source, item_start: TextSize) -> bool {
    let text = source.text();
    let line_start = text.line_start(item_start);
    let above = whitespace_start_before(source, line_start);
    leading_comment_block(source, text.line_start(above), line_start).is_some()
}

/// True when the line containing the dict's opening `{` carries a
/// trailing `# prose: keep` comment, the marker that pins a dict against
/// both entry reordering and module-constant banding.
pub(crate) fn has_keep_marker(source: &Source, dict: &ExprDict) -> bool {
    let line = source.text().full_line_range(dict.range().start());
    source
        .comment_ranges()
        .comments_in_range(line)
        .iter()
        .any(|c| source.slice(c).trim_start_matches('#').trim() == "prose: keep")
}

/// True when any line in the comment block reads as a section marker,
/// either a decorative rule line or a multi-hash heading.
pub(super) fn is_banner_block(source: &Source, block: TextRange) -> bool {
    source.slice(block).lines().any(is_marker_line)
}

/// Returns the range spanning every own-line comment between `lower`
/// and `upper`, from the first comment's line start to the last
/// comment's end, so a blank run dividing two comment runs falls
/// inside it. `None` when no own-line comment sits in that gap.
/// End-of-line comments on the predecessor's line are excluded.
pub(crate) fn leading_comment_block(
    source: &Source,
    lower: TextSize,
    upper: TextSize,
) -> Option<TextRange> {
    let text = source.text();
    let mut own_lines = source
        .comment_ranges()
        .comments_in_range(TextRange::new(lower, upper))
        .iter()
        .copied()
        .filter(|r| CommentRanges::is_own_line(r.start(), text));
    let first = own_lines.next()?;
    let last = own_lines.next_back().unwrap_or(first);
    Some(TextRange::new(text.line_start(first.start()), last.end()))
}

/// The whitespace run between the hash run of the comment at `range`
/// and its text, paired with the width that run settles to, which is
/// one space ahead of text and none where no text follows. `None` where
/// the opener passes through, covering an exempt leader and a
/// `columnar` comment whose run already opens on a space.
pub(crate) fn settled_opener(
    source: &Source,
    range: TextRange,
    columnar: bool,
) -> Option<(TextRange, usize)> {
    let body = source.slice(range).trim_start_matches('#');
    if body.starts_with(EXEMPT_LEADERS) {
        return None;
    }
    let text = body.trim_whitespace_start();
    if columnar && body.starts_with(' ') && !text.is_empty() {
        return None;
    }
    let opener = TextRange::at(
        range.end() - body.text_len(),
        body.text_len() - text.text_len(),
    );
    Some((opener, usize::from(!text.is_empty())))
}

/// The trailing comment on `offset`'s line, `None` where that line
/// carries no comment or carries an own-line one alone.
pub(crate) fn trailing_comment(source: &Source, offset: TextSize) -> Option<TextRange> {
    let line = source.text().full_line_range(offset);
    source
        .comment_ranges()
        .comments_in_range(line)
        .iter()
        .find(|comment| !CommentRanges::is_own_line(comment.start(), source.text()))
        .copied()
}

/// True when `line` opens with two or more `#`, the Markdown-style
/// heading shape that reads as a section divider.
fn is_heading_line(line: &str) -> bool {
    line.trim_start().starts_with("##")
}

/// True when `line` reads as a section marker, a decorative rule line or
/// a multi-hash heading.
fn is_marker_line(line: &str) -> bool {
    is_rule_line(line) || is_heading_line(line)
}

/// True for a character authors repeat to draw a divider rule.
fn is_rule_char(c: char) -> bool {
    matches!(c, '-' | '=' | '~' | '*' | '_' | '#' | '─' | '━' | '═')
}

/// True when `line` reads as a decorative rule, one repeated rule
/// character standing alone at five or more or flanking a label at three
/// or more, on whichever side of the label the author drew it. A closing
/// `#` caps a trailing run without breaking it, the box shape
/// `# Label ****#` takes. Box-drawing dashes count as rule characters.
fn is_rule_line(line: &str) -> bool {
    let body = line.trim_start().strip_prefix('#').map_or("", str::trim);
    let capped = body.strip_suffix('#').unwrap_or_default();
    let run = rule_run(body.chars())
        .max(rule_run(body.chars().rev()))
        .max(rule_run(capped.chars().rev()));
    run >= 5 || (run >= 3 && body.chars().count() > run)
}

/// The opening run of one repeated rule character in `chars`, zero when
/// it opens on anything else. Reversing the iterator measures the run
/// closing the same text.
fn rule_run(mut chars: impl Iterator<Item = char>) -> usize {
    match chars.next() {
        Some(first) if is_rule_char(first) => 1 + chars.take_while(|&c| c == first).count(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use unicode_width::UnicodeWidthStr;

    use super::*;
    use crate::testing::{notebook, parse};

    fn gap_block(s: &Source) -> Option<TextRange> {
        let body = &s.ast().body;
        leading_comment_block(s, body[0].end(), body[1].start())
    }

    #[rstest]
    #[case("x = 1\n# describes a\ndef a(): pass\n", false)]
    #[case("x = 1\n# --- Section ---\ndef a(): pass\n", true)]
    #[case("x = 1\n# prose: off\ndef a(): pass\n", true)]
    #[case("x = 1\n### Heading\ndef a(): pass\n", true)]
    #[case("x = 1\n# noqa: E501\ndef a(): pass\n", true)]
    #[case("if x:\n    pass\n    # type: ignore\ndef a(): pass\n", true)]
    fn anchors_in_place_spots_a_marker_a_directive_or_a_pragma(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        let block = gap_block(&s).expect("block");
        assert_eq!(anchors_in_place(&s, block), expected);
    }

    #[rstest]
    #[case("x = 1\n# describes a\ndef a(): pass\n", "# describes a")]
    #[case("x = 1\n# describes a\n\ndef a(): pass\n", "# describes a")]
    #[case("x = 1\n# one\n# two\n\ndef a(): pass\n", "# one\n# two")]
    #[case("x = 1\n# --- Section ---\ndef a(): pass\n", "")]
    #[case("x = 1\n# --- Section ---\n# describes a\ndef a(): pass\n", "")]
    #[case("x = 1\n# fmt: on\n\ndef a(): pass\n", "")]
    #[case("x = 1\n# type: ignore\n\ndef a(): pass\n", "")]
    #[case("if x:\n    pass\n    # describes a\ndef a(): pass\n", "")]
    #[case("x = 1\n\ndef a(): pass\n", "")]
    fn bound_block_start_binds_a_run_that_holds_no_anchor(#[case] src: &str, #[case] bound: &str) {
        let s = parse(src);
        let body = &s.ast().body;
        let item_start = body[1].start();
        let start = bound_block_start(&s, body[0].end(), item_start);
        assert_eq!(
            s.slice(TextRange::new(start, s.text().line_start(item_start)))
                .trim_end(),
            bound,
        );
    }

    #[test]
    fn bound_block_start_holds_at_a_member_sharing_its_line() {
        let s = parse("x = 1; y = 2\n");
        let body = &s.ast().body;
        let item_start = body[1].start();
        assert_eq!(
            bound_block_start(&s, body[0].end(), item_start),
            item_start,
            "a second statement on one line reaches back over nothing",
        );
    }

    #[rstest]
    #[case("# describes it\nimport os\n", true)]
    #[case("# describes it\n\nimport os\n", true)]
    #[case("# one\n# two\n\n\nimport os\n", true)]
    #[case("import os\n", false)]
    #[case("x = 1\n\nimport os\n", false)]
    #[case("x = 1  # trail\n\nimport os\n", false)]
    #[case("# far above\nx = 1\n\nimport os\n", false)]
    fn comment_leads_reaches_a_block_across_the_blank_run(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        let item = s.ast().body.last().expect("a statement");
        assert_eq!(comment_leads(&s, item.start()), expected);
    }

    #[rstest]
    #[case::across_a_blank_run(&["# describes it\n", "import os"])]
    #[case::written_tight(&["# describes it", "import os"])]
    fn comment_leads_stops_at_a_notebook_cell_wall(#[case] cells: &[&str]) {
        let s = notebook(cells);
        let item = s.ast().body.last().expect("a statement");
        assert!(!comment_leads(&s, item.start()));
    }

    #[rstest]
    #[case::rule_line(
        "x = 1\n# ========================\n# Section: helpers\n# ========================\ndef f(): pass\n",
        true
    )]
    #[case::hash_heading("x = 1\n### Codec APIs\ndef f(): pass\n", true)]
    #[case::heading_below_prose(
        "x = 1\n# see the module docs\n### API Reference\ndef f(): pass\n",
        true
    )]
    #[case::all_prose("x = 1\n# describes f\n# helper\ndef f(): pass\n", false)]
    fn is_banner_block_reads_a_rule_line_or_a_hash_heading(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        let block = gap_block(&s).expect("block");
        assert_eq!(is_banner_block(&s, block), expected);
    }

    #[rstest]
    fn is_heading_line_accepts_two_or_more_hashes(
        #[values("## heading", "### Codec APIs", "#### deep", "  ## indented")] line: &str,
    ) {
        assert!(is_heading_line(line));
    }

    #[rstest]
    fn is_heading_line_rejects_single_hash(
        #[values("# describes f", "#", "#!/usr/bin/env python", "#%%")] line: &str,
    ) {
        assert!(!is_heading_line(line));
    }

    #[rstest]
    fn is_rule_line_accepts_box_drawing_runs(
        #[values("# ─────", "# ━━━━━", "# ═════")] line: &str,
    ) {
        assert!(is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_accepts_canonical_decorative_runs(
        #[values("# =====", "# -----", "# *****", "# _____", "# ~~~~~", "##########")] line: &str,
    ) {
        assert!(is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_accepts_flanked_label(
        #[values(
            "# --- Lifecycle ---",
            "# === Section ===",
            "# ─── Box ───",
            "# *** Note ***"
        )]
        line: &str,
    ) {
        assert!(is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_accepts_trailing_rule(
        #[values(
            "# Sequence Operations *********#",
            "# Loaders ######################",
            "# -- Public interface ---------",
            "# ─── Box ───────────────"
        )]
        line: &str,
    ) {
        assert!(is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_alpha_prose(
        #[values("# describes f", "# Section: helpers", "# x")] line: &str,
    ) {
        assert!(!is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_mixed_characters(
        #[values("# = = = =", "# -=-=-=", "# - - -", "# -*- coding: utf-8 -*-")] line: &str,
    ) {
        assert!(!is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_short_runs(#[values("# ====", "# ---", "# ", "#")] line: &str) {
        assert!(!is_rule_line(line));
    }

    #[test]
    fn leading_comment_block_returns_block_for_chain_of_own_line_comments() {
        let s = parse("x = 1\n# a\n# b\ndef f(): pass\n");
        let block = gap_block(&s).expect("block");
        let comments = s.comment_ranges();
        assert_eq!(block.start(), s.text().line_start(comments[0].start()));
        assert_eq!(block.end(), comments[1].end());
    }

    #[rstest]
    #[case::no_own_line_comment("x = 1\ndef f(): pass\n")]
    #[case::trailing_comment_only("x = 1  # trail\ndef f(): pass\n")]
    fn leading_comment_block_returns_none_without_an_own_line_comment(#[case] src: &str) {
        assert!(gap_block(&parse(src)).is_none());
    }

    #[rstest]
    #[case::tight_opener_gains_a_space("x = 1  #note\n", false, Some((0, 1)))]
    #[case::settled_opener_holds("x = 1  # note\n", false, Some((1, 1)))]
    #[case::wide_opener_collapses("x = 1  #    note\n", false, Some((4, 1)))]
    #[case::empty_comment_sheds_its_run("x = 1  #   \n", false, Some((3, 0)))]
    #[case::columnar_run_holds_its_indent("#     deep\n", true, None)]
    #[case::columnar_bare_hash_still_sheds("#\n", true, Some((0, 0)))]
    #[case::exempt_leader_passes_through("x = 1  #: attribute\n", false, None)]
    fn settled_opener_reads_the_run_the_rule_settles(
        #[case] src: &str,
        #[case] columnar: bool,
        #[case] expected: Option<(usize, usize)>,
    ) {
        let source = parse(src);
        let comment = source.comment_ranges()[0];
        let read = settled_opener(&source, comment, columnar)
            .map(|(opener, settled)| (source.slice(opener).width(), settled));
        assert_eq!(read, expected);
    }

    #[rstest]
    #[case::trailing("x = 1  # note\n", Some("# note"))]
    #[case::own_line("# note\nx = 1\n", None)]
    #[case::no_comment("x = 1\n", None)]
    fn trailing_comment_answers_only_a_comment_sharing_its_code_row(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let start = source.ast().body[0].start();
        assert_eq!(
            trailing_comment(&source, start).map(|range| source.slice(range)),
            expected,
        );
    }
}
