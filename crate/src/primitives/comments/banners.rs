//! Reads a comment block as a section divider rather than a description.

use ruff_text_size::TextRange;

use super::*;

/// True when any line in the comment block reads as a section marker,
/// either a decorative rule line or a multi-hash heading.
pub(crate) fn is_banner_block(source: &Source, block: TextRange) -> bool {
    source
        .slice(block)
        .universal_newlines()
        .any(|line| is_marker_line(&line))
}

/// True when `line` opens with two or more `#`, the Markdown-style
/// heading shape that reads as a section divider.
fn is_heading_line(line: &str) -> bool {
    line.trim_start().starts_with("##")
}

/// True when `line` reads as a section marker, a decorative rule line or
/// a multi-hash heading.
pub(super) fn is_marker_line(line: &str) -> bool {
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

    use super::*;
    use crate::testing::parse;

    fn gap_block(s: &Source) -> Option<TextRange> {
        let body = &s.ast().body;
        leading_comment_block(s, body[0].end(), body[1].start())
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
}
