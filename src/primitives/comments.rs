//! Own-line comment-block detection between two statements, covering
//! the contiguous leading block and whether it reads as a decorative
//! banner or a multi-hash heading.

use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{TextRange, TextSize};

use crate::source::Source;

/// True when any line in the comment block reads as a section marker,
/// either a decorative rule line or a multi-hash heading.
pub(crate) fn is_banner_block(source: &Source, block: TextRange) -> bool {
    source.slice(block).lines().any(is_marker_line)
}

/// True when `line` reads as a section marker, a decorative rule line or
/// a multi-hash heading.
pub(crate) fn is_marker_line(line: &str) -> bool {
    is_rule_line(line) || is_heading_line(line)
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

/// True when `line` opens with two or more `#`, the Markdown-style
/// heading shape that reads as a section divider.
fn is_heading_line(line: &str) -> bool {
    line.trim_start().starts_with("##")
}

/// True for a character authors repeat to draw a divider rule.
fn is_rule_char(c: char) -> bool {
    matches!(c, '-' | '=' | '~' | '*' | '_' | '#' | '─' | '━' | '═')
}

/// True when `line` reads as a decorative rule, either a pure run of five
/// or more identical rule characters or a run of three or more flanking a
/// label. Box-drawing dashes count as rule characters.
fn is_rule_line(line: &str) -> bool {
    let body = line.trim_start().strip_prefix('#').map_or("", str::trim);
    let mut chars = body.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_rule_char(first) {
        return false;
    }
    let run = 1 + chars.take_while(|&c| c == first).count();
    run >= 5 || (run >= 3 && body.chars().count() > run)
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
    fn is_rule_line_rejects_alpha_prose(
        #[values("# describes f", "# Section: helpers", "# x")] line: &str,
    ) {
        assert!(!is_rule_line(line));
    }

    #[rstest]
    fn is_rule_line_rejects_mixed_characters(
        #[values("# = = = =", "# -=-=-=", "# - - -")] line: &str,
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
}
