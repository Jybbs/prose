//! Google-style section parsing: Title-case headings and their
//! `name: description` entries grouped per section.

use std::sync::LazyLock;

use regex_lite::Regex;
use ruff_python_ast::StringLiteral;
use ruff_python_trivia::leading_indentation;
use ruff_source_file::{Line, UniversalNewlineIterator};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::body::{DocstringBody, indent_prefix, triple_quoted_body};
use super::scan::{LineScan, LineScanner};
use crate::source::Source;

static ENTRY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\w[\w.]*\s*:\s+\S").expect("static pattern compiles"));

static SECTION_HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Z][A-Za-z]*( [A-Z][A-Za-z]*)*:").expect("static pattern compiles")
});

/// One `name: description` entry inside a Google-style section. The
/// range covers the entry's head line through the last continuation
/// line attached to it (verbatim region, hanging description, list
/// item), excluding the trailing newline.
pub(crate) struct SectionEntry<'a> {
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
}

impl Ranged for SectionEntry<'_> {
    fn range(&self) -> TextRange {
        self.range
    }
}

struct EntryWalker<'src> {
    open_entry: Option<SectionEntry<'src>>,
    open_section: Option<Vec<SectionEntry<'src>>>,
    scanner: LineScanner,
    sections: Vec<Vec<SectionEntry<'src>>>,
}

impl<'src> EntryWalker<'src> {
    fn new(body_indent_chars: usize) -> Self {
        Self {
            open_entry: None,
            open_section: None,
            scanner: LineScanner::new(body_indent_chars),
            sections: Vec::new(),
        }
    }

    fn consume(&mut self, line: Line<'src>) {
        let line_start = line.start();
        let line_end = line.end();
        let text = line.as_str();

        let indent_str = leading_indentation(text);
        let trimmed = &text[indent_str.len()..];
        let indent_chars = indent_str.chars().count();

        match self.scanner.classify(trimmed, indent_chars) {
            LineScan::Blank => {}
            LineScan::Body => self.consume_body(line_start, line_end, trimmed, indent_chars),
            LineScan::Fence
            | LineScan::InFence
            | LineScan::ListContinuation
            | LineScan::ListMarker
            | LineScan::Verbatim
            | LineScan::VerbatimOpen => {
                self.extend_open_entry(line_end);
            }
        }
    }

    fn consume_body(
        &mut self,
        line_start: TextSize,
        line_end: TextSize,
        trimmed: &'src str,
        indent_chars: usize,
    ) {
        let body_indent = self.scanner.body_indent_chars();
        if indent_chars == body_indent {
            self.finish_section();
            if section_heading(trimmed) {
                self.open_section = Some(Vec::new());
            }
            return;
        }
        if self.open_section.is_none() {
            return;
        }
        if indent_chars == body_indent + 4 && entry_description_col(trimmed).is_some() {
            self.finish_entry();
            self.open_entry = Some(SectionEntry {
                name: entry_name(trimmed),
                range: TextRange::new(line_start, line_end),
            });
            return;
        }
        self.extend_open_entry(line_end);
    }

    fn extend_open_entry(&mut self, line_end: TextSize) {
        if let Some(entry) = self.open_entry.as_mut() {
            entry.range = TextRange::new(entry.range.start(), line_end);
        }
    }

    fn finish_entry(&mut self) {
        let Some(entry) = self.open_entry.take() else {
            return;
        };
        self.open_section
            .as_mut()
            .expect("open_entry only set while open_section is Some")
            .push(entry);
    }

    fn finish_section(&mut self) {
        self.finish_entry();
        if let Some(entries) = self.open_section.take().filter(|e| !e.is_empty()) {
            self.sections.push(entries);
        }
    }
}

/// Walks the entry-carrying Google-style sections in `lit`'s body
/// and returns each section's entries with source-relative byte
/// ranges. Returns an empty vector when `lit` carries no body
/// (single-line, non-triple-quoted, or no `\n`), no entry-carrying
/// section heading, or no recognized entries within those sections.
/// Each entry's range covers its head line through any attached
/// continuation lines (hanging description, indented code, list
/// item, fenced code block).
pub(crate) fn entry_carrying_sections<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Vec<SectionEntry<'src>>> {
    let Some(body) = triple_quoted_body(source, lit).filter(DocstringBody::is_multiline) else {
        return Vec::new();
    };
    let mut walker = EntryWalker::new(indent_prefix(source, lit).chars().count());
    for line in UniversalNewlineIterator::with_offset(body.text, body.range.start()) {
        walker.consume(line);
    }
    walker.finish_section();
    walker.sections
}

/// Returns the description-start character column when `trimmed`
/// matches the `name: description` shape of a Google-style section
/// entry head. `None` for any line that does not match.
pub(crate) fn entry_description_col(trimmed: &str) -> Option<usize> {
    let m = ENTRY_PATTERN.find(trimmed)?;
    Some(trimmed[..m.end() - 1].chars().count())
}

/// True when `trimmed` opens with a Title-case word or multi-word
/// run with every word capitalized, immediately followed by `:`.
/// Trailing content after the `:` is permitted.
pub(crate) fn section_heading(trimmed: &str) -> bool {
    SECTION_HEADING.is_match(trimmed)
}

/// Returns the entry-name prefix of `trimmed` for a line already
/// matched by [`entry_description_col`]. Stops at the first `:`,
/// trimming any trailing whitespace.
fn entry_name(trimmed: &str) -> &str {
    let colon = trimmed.find(':').expect("entry head carries a colon");
    trimmed[..colon].trim_end()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        primitives::docstring::body_docstring,
        testing::{first_def, parse},
    };

    fn entry_names<'a>(sections: &[Vec<SectionEntry<'a>>]) -> Vec<Vec<&'a str>> {
        sections
            .iter()
            .map(|s| s.iter().map(|e| e.name).collect())
            .collect()
    }

    fn first_function_docstring(source: &Source) -> &StringLiteral {
        body_docstring(&first_def(source).body)
            .expect("function body starts with a docstring literal")
    }

    #[test]
    fn entry_carrying_sections_attaches_fenced_code_block_to_parent_entry() {
        let src = "def f():\n    \"\"\"\n    Raises:\n        ValueError: malformed input::\n\n            ```python\n            raise ValueError(\"bad\")\n            ```\n\n        OSError: io trouble.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["ValueError", "OSError"]]);
        let value_error_slice = s.slice(sections[0][0].range);
        assert!(value_error_slice.contains("```python"));
        assert!(value_error_slice.contains("raise ValueError"));
    }

    #[test]
    fn entry_carrying_sections_attaches_list_continuation_to_parent_entry() {
        let src = "def f():\n    \"\"\"\n    Args:\n        foo: takes a list::\n\n            - item one\n              still item one\n            - item two\n\n        bar: another arg.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["foo", "bar"]]);
        let foo_slice = s.slice(sections[0][0].range);
        assert!(foo_slice.contains("- item one"));
        assert!(foo_slice.contains("still item one"));
        assert!(foo_slice.contains("- item two"));
    }

    #[test]
    fn entry_carrying_sections_attaches_verbatim_continuation_to_parent_entry() {
        let src = "def f():\n    \"\"\"\n    Raises:\n        ValueError: malformed::\n\n            >>> sample\n\n        OSError: io trouble.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["ValueError", "OSError"]]);
        let value_error_slice = s.slice(sections[0][0].range);
        assert!(value_error_slice.contains(">>> sample"));
    }

    #[test]
    fn entry_carrying_sections_drops_empty_args_section_with_no_entries() {
        let src = "def f():\n    \"\"\"\n    Args:\n        Just prose without a name and colon.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        assert!(entry_carrying_sections(&s, lit).is_empty());
    }

    #[test]
    fn entry_carrying_sections_groups_entries_per_section() {
        let src = "def f():\n    \"\"\"\n    Args:\n        b: one\n        a: two\n\n    Returns:\n        z: three\n        y: four\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["b", "a"], vec!["z", "y"]]);
    }

    #[test]
    fn entry_carrying_sections_recognizes_section_by_content_shape() {
        let src = "def f():\n    \"\"\"\n    Steps:\n        bar: second\n        alpha: first\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["bar", "alpha"]]);
    }

    #[test]
    fn entry_carrying_sections_returns_empty_for_section_without_entries() {
        let src = "def f():\n    \"\"\"\n    Returns:\n        Just prose without a name and colon.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        assert!(entry_carrying_sections(&s, lit).is_empty());
    }

    #[test]
    fn entry_carrying_sections_returns_empty_for_single_line_docstring() {
        let src = "def f():\n    \"\"\"Args: foo: bar\"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        assert!(entry_carrying_sections(&s, lit).is_empty());
    }

    #[test]
    fn entry_carrying_sections_walks_opener_on_same_line_docstring() {
        let src = "def f():\n    \"\"\"Summary.\n\n    Args:\n        bar: two\n        alpha: one\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["bar", "alpha"]]);
    }

    #[test]
    fn entry_carrying_sections_yields_single_entry_section() {
        let src = "def f():\n    \"\"\"\n    Returns:\n        value: the result.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["value"]]);
    }

    #[test]
    fn entry_description_col_rejects_lines_without_name_colon_shape() {
        assert!(entry_description_col("just prose with no colon").is_none());
        assert!(entry_description_col("name:no_space_after_colon").is_none());
        assert!(entry_description_col(": no name before colon").is_none());
        assert!(entry_description_col("name: ").is_none());
        assert!(entry_description_col("123: digits-only name").is_some());
    }

    #[test]
    fn entry_description_col_returns_char_column_of_description_start() {
        assert_eq!(entry_description_col("name: desc"), Some(6));
        assert_eq!(entry_description_col("name : desc"), Some(7));
        assert_eq!(entry_description_col("dotted.name: desc"), Some(13));
    }

    #[rstest]
    fn section_heading_accepts_title_case_word_with_colon(
        #[values(
            "Args:",
            "Attributes:",
            "Raises:",
            "Returns:",
            "Yields:",
            "Examples:",
            "Note:",
            "Warning:",
            "Arguments:",
            "Parameters:",
            "Inputs:",
            "Steps:",
            "Outputs:"
        )]
        heading: &str,
    ) {
        assert!(section_heading(heading));
    }

    #[test]
    fn section_heading_accepts_multi_word_title_case_with_colon() {
        assert!(section_heading("Other Parameters:"));
        assert!(section_heading("See Also:"));
        assert!(section_heading("Side Effects:"));
    }

    #[test]
    fn section_heading_accepts_trailing_content_after_colon() {
        assert!(section_heading("Returns: int"));
        assert!(section_heading("Note: see below"));
    }

    #[test]
    fn section_heading_rejects_lowercase_start_or_missing_colon() {
        assert!(!section_heading("args:"));
        assert!(!section_heading("Args :"));
        assert!(!section_heading("Args"));
        assert!(!section_heading("Foo bar:"));
        assert!(!section_heading("1Args:"));
        assert!(!section_heading(": no name"));
    }
}
