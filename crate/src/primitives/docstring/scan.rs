//! Line classification for a docstring body: fences, blanks, list
//! markers, section underlines, table rows and rules, comment-led
//! code examples, doctest blocks, reStructuredText field lists,
//! Sphinx directives, and their continuations.

use ruff_python_trivia::{PythonWhitespace, leading_indentation};

/// The classification of a docstring body line by the shared fence,
/// blank, list, and verbatim scanner. Every variant but `Body` is
/// terminal for the line, with `Body` handed to the walker's own
/// dispatch. `VerbatimOpen` either stands alone for its line or
/// opens a region running to the next blank, and `Verbatim` carries
/// a line inside an open region.
#[derive(Debug)]
pub(crate) enum LineScan {
    Blank,
    Body,
    Fence,
    InFence,
    ListContinuation,
    ListMarker,
    Verbatim,
    VerbatimOpen,
}

/// The fence, list, and verbatim-block state a docstring walker
/// carries across lines. [`LineScanner::scan_line`] advances the state
/// per line and returns its [`ScannedLine`], leaving each walker to
/// dispatch its own effect.
pub(crate) struct LineScanner {
    body_indent_chars: usize,
    in_block: bool,
    in_fence: bool,
    list_indent: Option<usize>,
}

impl LineScanner {
    pub(crate) fn new(body_indent_chars: usize) -> Self {
        Self {
            body_indent_chars,
            in_block: false,
            in_fence: false,
            list_indent: None,
        }
    }

    fn classify(&mut self, trimmed: &str, indent_chars: usize) -> LineScan {
        if trimmed.starts_with("```") {
            self.in_fence = !self.in_fence;
            self.list_indent = None;
            return LineScan::Fence;
        }
        if self.in_fence {
            return LineScan::InFence;
        }
        if self.in_block && !trimmed.is_empty() {
            return LineScan::Verbatim;
        }
        if trimmed.is_empty() {
            self.in_block = false;
            self.list_indent = None;
            return LineScan::Blank;
        }
        if let Some(marker) = self.list_indent {
            if indent_chars > marker {
                return LineScan::ListContinuation;
            }
            self.list_indent = None;
        }
        if indent_chars >= self.body_indent_chars {
            if is_list_marker(trimmed) {
                self.list_indent = Some(indent_chars);
                return LineScan::ListMarker;
            }
            if is_grid_table_line(trimmed) || is_section_underline(trimmed) {
                return LineScan::VerbatimOpen;
            }
            if is_comment_marker(trimmed)
                || is_directive(trimmed)
                || is_doctest_prompt(trimmed)
                || is_field_marker(trimmed)
                || is_simple_table_rule(trimmed)
            {
                self.in_block = true;
                return LineScan::VerbatimOpen;
            }
        }
        LineScan::Body
    }

    pub(crate) fn body_indent_chars(&self) -> usize {
        self.body_indent_chars
    }

    /// Splits `line` into its indent prefix and trimmed body, then
    /// classifies the body, so a walker reads geometry and scan from
    /// one call.
    pub(crate) fn scan_line<'a>(&mut self, line: &'a str) -> ScannedLine<'a> {
        let indent = leading_indentation(line);
        let trimmed = line[indent.len()..].trim_whitespace_end();
        let indent_chars = indent.chars().count();
        let scan = self.classify(trimmed, indent_chars);
        ScannedLine {
            indent,
            indent_chars,
            scan,
            trimmed,
        }
    }

    /// The character column of a section's entry heads and body
    /// prose, one 4-space step past the body indent.
    pub(crate) fn section_body_indent_chars(&self) -> usize {
        self.body_indent_chars + 4
    }
}

/// A docstring body line's geometry paired with its classification:
/// the indent prefix, its character width, the trimmed body, and the
/// [`LineScan`] the scanner advanced to.
pub(crate) struct ScannedLine<'a> {
    pub(crate) indent: &'a str,
    pub(crate) indent_chars: usize,
    pub(crate) scan: LineScan,
    pub(crate) trimmed: &'a str,
}

/// True when `trimmed` is a delimited head, an `open` prefix then a
/// non-empty name run then a `close` delimiter.
fn head_delimited(trimmed: &str, open: &str, close: &str) -> bool {
    trimmed
        .strip_prefix(open)
        .and_then(|rest| rest.split_once(close))
        .is_some_and(|(name, _)| !name.is_empty())
}

/// True when `trimmed` opens a comment-led code example, the `# `
/// prefix a Python comment carries.
fn is_comment_marker(trimmed: &str) -> bool {
    trimmed.starts_with("# ")
}

/// True when `trimmed` opens a reStructuredText directive, a `.. `
/// prefix then a name then a `::` close.
fn is_directive(trimmed: &str) -> bool {
    head_delimited(trimmed, ".. ", "::")
}

/// True when `trimmed` opens an interactive doctest example, the
/// `>>> ` prompt marker.
fn is_doctest_prompt(trimmed: &str) -> bool {
    trimmed.starts_with(">>> ")
}

/// True when `trimmed` is a reStructuredText field-list head, an
/// opening `:` then a non-colon name run then a closing `:`.
fn is_field_marker(trimmed: &str) -> bool {
    head_delimited(trimmed, ":", ":")
}

/// True when `trimmed` is a grid-table line, either a row opening with
/// the `|` cell delimiter or a rule built from `+` corners joined by
/// runs of `-` or `=`.
fn is_grid_table_line(trimmed: &str) -> bool {
    trimmed.starts_with('|')
        || (trimmed.starts_with('+')
            && trimmed.contains(['-', '='])
            && trimmed.bytes().all(|b| matches!(b, b'+' | b'-' | b'=')))
}

/// True when `trimmed` opens with a Markdown list marker (`-`, `*`,
/// or `+` followed by a space) or a numeric marker (one or more
/// digits followed by `. `). Used by the shared line scanner to
/// recognize verbatim-passthrough list items.
fn is_list_marker(trimmed: &str) -> bool {
    if trimmed
        .strip_prefix(['-', '*', '+'])
        .is_some_and(|rest| rest.starts_with(' '))
    {
        return true;
    }
    let after_digits = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    after_digits.len() < trimmed.len() && after_digits.starts_with(". ")
}

/// True when `trimmed` is a section underline, a run of one repeated
/// adornment character (`-`, `=`, or `~`).
fn is_section_underline(trimmed: &str) -> bool {
    let mut chars = trimmed.chars();
    chars
        .next()
        .is_some_and(|first| matches!(first, '-' | '=' | '~') && chars.all(|c| c == first))
}

/// True when `trimmed` is a simple-table rule, two or more runs of one
/// adornment character (`-` or `=`) separated by spaces.
fn is_simple_table_rule(trimmed: &str) -> bool {
    let Some(adornment) = trimmed.chars().next().filter(|c| matches!(c, '-' | '=')) else {
        return false;
    };
    trimmed.contains(' ') && trimmed.chars().all(|c| c == adornment || c == ' ')
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

    #[rstest]
    fn classify_runs_a_verbatim_block_through_to_blank(
        #[values(
            ">>> add(1, 2)",
            "# sum the rows",
            ".. note:: text",
            ":param x: input",
            "=====  ====="
        )]
        opener: &str,
    ) {
        let mut scanner = LineScanner::new(0);
        assert_matches!(scanner.classify(opener, 0), LineScan::VerbatimOpen);
        assert_matches!(scanner.classify("total = sum(rows)", 0), LineScan::Verbatim);
        assert_matches!(scanner.classify(">>> add(3, 4)", 0), LineScan::Verbatim);
        assert_matches!(scanner.classify("7", 0), LineScan::Verbatim);
        assert_matches!(scanner.classify("", 0), LineScan::Blank);
        assert_matches!(scanner.classify("Back to prose.", 0), LineScan::Body);
    }

    #[test]
    fn is_comment_marker_matches_hash_followed_by_a_space() {
        assert!(is_comment_marker("# compute the total"));
        assert!(!is_comment_marker("#1. open the file"));
        assert!(!is_comment_marker("#"));
        assert!(!is_comment_marker("plain prose"));
    }

    #[test]
    fn is_directive_matches_dotdot_name_double_colon() {
        assert!(is_directive(".. versionadded:: 0.10"));
        assert!(is_directive(".. deprecated:: 1.0"));
        assert!(!is_directive(".. a plain comment"));
        assert!(!is_directive(".. :: no name"));
        assert!(!is_directive("..versionadded:: 0.10"));
    }

    #[test]
    fn is_doctest_prompt_matches_chevrons_and_space() {
        assert!(is_doctest_prompt(">>> add(2, 3)"));
        assert!(!is_doctest_prompt(">>>add(2, 3)"));
        assert!(!is_doctest_prompt("... continuation"));
        assert!(!is_doctest_prompt("plain prose"));
    }

    #[test]
    fn is_field_marker_matches_colon_name_colon() {
        assert!(is_field_marker(":codeauthor: name"));
        assert!(is_field_marker(":maturity:   new"));
        assert!(is_field_marker(":param x: the input"));
        assert!(!is_field_marker("::"));
        assert!(!is_field_marker(":no closing colon"));
        assert!(!is_field_marker("name: value"));
    }

    #[test]
    fn is_grid_table_line_matches_pipe_rows_and_corner_rules() {
        assert!(is_grid_table_line("| Python | JSON |"));
        assert!(is_grid_table_line("| line block continuation"));
        assert!(is_grid_table_line("+--------+------+"));
        assert!(is_grid_table_line("+========+======+"));
        assert!(!is_grid_table_line("+ list item"));
        assert!(!is_grid_table_line("++++"));
        assert!(!is_grid_table_line("Accepts int | None."));
        assert!(!is_grid_table_line("plain prose"));
    }

    #[test]
    fn is_list_marker_matches_dash_star_plus_and_numeric() {
        assert!(is_list_marker("- foo"));
        assert!(is_list_marker("* foo"));
        assert!(is_list_marker("+ foo"));
        assert!(is_list_marker("1. foo"));
        assert!(is_list_marker("12. foo"));
        assert!(!is_list_marker("foo"));
        assert!(!is_list_marker("-foo"));
        assert!(!is_list_marker(". foo"));
    }

    #[test]
    fn is_section_underline_matches_repeated_adornment() {
        assert!(is_section_underline("----------"));
        assert!(is_section_underline("======"));
        assert!(is_section_underline("~~~"));
        assert!(!is_section_underline("-=-=-"));
        assert!(!is_section_underline("--- text"));
        assert!(!is_section_underline("Parameters"));
    }

    #[test]
    fn is_simple_table_rule_matches_spaced_adornment_runs() {
        assert!(is_simple_table_rule("=====  ====="));
        assert!(is_simple_table_rule("-----  ---  --"));
        assert!(!is_simple_table_rule("====="));
        assert!(!is_simple_table_rule("=====  -----"));
        assert!(!is_simple_table_rule("~~~  ~~~"));
        assert!(!is_simple_table_rule("Name   Meaning"));
        assert!(!is_simple_table_rule("--- text"));
    }

    #[test]
    fn scan_line_trims_trailing_whitespace_before_classifying() {
        let mut scanner = LineScanner::new(0);
        assert_matches!(
            scanner.scan_line("+---+---+   ").scan,
            LineScan::VerbatimOpen
        );
    }
}
