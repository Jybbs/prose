//! Detection of the own-line comment block between two statements
//! and whether it reads as a banner or as prose.

use ruff_python_ast::Stmt;
use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::source::Source;

/// True when any line in the comment block reads as a divider, either a
/// decorative rule line or a multi-hash heading.
pub(super) fn is_banner_block(source: &Source, block: TextRange) -> bool {
    source
        .slice(block)
        .lines()
        .any(|line| is_rule_line(line) || is_heading_line(line))
}

/// Returns the contiguous range of own-line comments lying between
/// `prev_end` and `curr.start()`. `None` when no own-line comment
/// sits in that gap. End-of-line comments on the predecessor's line
/// are excluded.
pub(super) fn leading_block_of(
    source: &Source,
    prev_end: TextSize,
    curr: &Stmt,
) -> Option<TextRange> {
    let text = source.text();
    let mut own_lines = source
        .comment_ranges()
        .comments_in_range(TextRange::new(prev_end, curr.start()))
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

/// True when `line` is a comment whose body, after stripping the
/// leading `#` and surrounding whitespace, consists of 5 or more
/// identical non-alphanumeric characters.
fn is_rule_line(line: &str) -> bool {
    let stripped = line.trim_start().strip_prefix('#').map_or("", str::trim);
    let bytes = stripped.as_bytes();
    bytes.len() >= 5 && !bytes[0].is_ascii_alphanumeric() && bytes.iter().all(|&b| b == bytes[0])
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn is_banner_block_detects_block_with_any_rule_line() {
        let s = parse(
            "x = 1\n# ========================\n# Section: helpers\n# ========================\ndef f(): pass\n",
        );
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        assert!(is_banner_block(&s, block));
    }

    #[test]
    fn is_banner_block_detects_block_with_hash_heading() {
        let s = parse("x = 1\n### Codec APIs\ndef f(): pass\n");
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        assert!(is_banner_block(&s, block));
    }

    #[test]
    fn is_banner_block_returns_false_for_all_prose_block() {
        let s = parse("x = 1\n# describes f\n# helper\ndef f(): pass\n");
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
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
    fn is_rule_line_accepts_canonical_decorative_runs(
        #[values("# =====", "# -----", "# *****", "# _____", "# ~~~~~", "##########")] line: &str,
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
    fn leading_block_of_returns_block_for_chain_of_own_line_comments() {
        let s = parse("x = 1\n# a\n# b\ndef f(): pass\n");
        let body = &s.ast().body;
        let block = leading_block_of(&s, body[0].end(), &body[1]).expect("block");
        let comments = s.comment_ranges();
        assert_eq!(block.start(), s.text().line_start(comments[0].start()));
        assert_eq!(block.end(), comments[1].end());
    }

    #[test]
    fn leading_block_of_returns_none_when_no_own_line_comments_between() {
        let s = parse("x = 1\ndef f(): pass\n");
        let body = &s.ast().body;
        assert!(leading_block_of(&s, body[0].end(), &body[1]).is_none());
    }

    #[test]
    fn leading_block_of_skips_trailing_end_of_line_comments() {
        let s = parse("x = 1  # trail\ndef f(): pass\n");
        let body = &s.ast().body;
        assert!(leading_block_of(&s, body[0].end(), &body[1]).is_none());
    }
}
