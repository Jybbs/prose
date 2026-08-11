//! Own-line comment-block detection between two statements, covering
//! the leading block, whether it reads as a decorative banner or a
//! multi-hash heading, and where the block binding to the member below
//! it starts. A run anchors in place on a section marker, a suppression
//! directive, or a tool pragma, and binds to the member otherwise,
//! whatever blank line sits between the two. The trailing-comment gap
//! the banding and spacing rules both seat lives here as well.

use ruff_python_ast::ExprDict;
use ruff_python_trivia::{CommentRanges, is_pragma_comment};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{source::Source, suppression::is_directive_comment};

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
/// member leaves the run bound, matching the gap `blank-lines` settles
/// beneath a description.
pub(crate) fn bound_block_start(
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
pub(crate) fn is_banner_block(source: &Source, block: TextRange) -> bool {
    source.slice(block).lines().any(is_marker_line)
}

/// Returns the contiguous range of own-line comments lying between
/// `lower` and `upper`. `None` when no own-line comment sits in that
/// gap. End-of-line comments on the predecessor's line are excluded.
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

/// The start of the trailing comment on `offset`'s line, `None` where
/// that line carries no comment or carries an own-line one alone.
pub(crate) fn trailing_comment_start(source: &Source, offset: TextSize) -> Option<TextSize> {
    let line = source.text().full_line_range(offset);
    source
        .comment_ranges()
        .comments_in_range(line)
        .iter()
        .map(Ranged::start)
        .find(|start| !CommentRanges::is_own_line(*start, source.text()))
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
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::parse;

    fn gap_block(s: &Source) -> Option<TextRange> {
        let body = &s.ast().body;
        leading_comment_block(s, body[0].end(), body[1].start())
    }

    #[test]
    fn is_banner_block_detects_block_with_any_rule_line() {
        let s = parse(
            "x = 1\n# ========================\n# Section: helpers\n# ========================\ndef f(): pass\n",
        );
        let block = gap_block(&s).expect("block");
        assert!(is_banner_block(&s, block));
    }

    #[test]
    fn is_banner_block_detects_block_with_hash_heading() {
        let s = parse("x = 1\n### Codec APIs\ndef f(): pass\n");
        let block = gap_block(&s).expect("block");
        assert!(is_banner_block(&s, block));
    }

    #[test]
    fn is_banner_block_detects_heading_on_non_leading_line() {
        let s = parse("x = 1\n# see the module docs\n### API Reference\ndef f(): pass\n");
        let block = gap_block(&s).expect("block");
        assert!(is_banner_block(&s, block));
    }

    #[test]
    fn is_banner_block_returns_false_for_all_prose_block() {
        let s = parse("x = 1\n# describes f\n# helper\ndef f(): pass\n");
        let block = gap_block(&s).expect("block");
        assert!(!is_banner_block(&s, block));
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

    #[test]
    fn leading_comment_block_returns_none_when_no_own_line_comments_between() {
        let s = parse("x = 1\ndef f(): pass\n");
        assert!(gap_block(&s).is_none());
    }

    #[test]
    fn leading_comment_block_skips_trailing_end_of_line_comments() {
        let s = parse("x = 1  # trail\ndef f(): pass\n");
        assert!(gap_block(&s).is_none());
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
}
