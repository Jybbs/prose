//! Google-style section walking: the entries of each Title-case-headed
//! section, with the continuation lines attached to each entry. The
//! per-line grammar the walk dispatches on lives in `grammar`.

use ruff_python_ast::StringLiteral;
use ruff_source_file::{Line, UniversalNewlineIterator};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::body::{DocstringBody, triple_quoted_body};
use super::grammar::{section_heading, sibling_entry_head};
use super::scan::{LineScan, LineScanner, ScannedLine};
use crate::source::Source;

/// One `name: description` entry inside a Google-style section. The
/// range covers the entry's head line through the last continuation
/// line attached to it (verbatim region, hanging description, list
/// item), excluding the trailing newline. `colon` is the source offset
/// of the head line's separating `:`.
pub(crate) struct SectionEntry<'a> {
    pub(crate) colon: TextSize,
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
        let ScannedLine {
            indent,
            indent_chars,
            scan,
            trimmed,
        } = self.scanner.scan_line(line.as_str());

        match scan {
            LineScan::Blank => {}
            LineScan::Body => {
                self.consume_body(line_start, line_end, indent, trimmed, indent_chars);
            }
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
        indent: &str,
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
        if let Some(head) = sibling_entry_head(
            indent_chars,
            self.scanner.section_body_indent_chars(),
            trimmed,
        ) {
            self.finish_entry();
            self.open_entry = Some(SectionEntry {
                colon: line_start + TextSize::of(indent) + TextSize::of(&trimmed[..head.colon]),
                name: head.name,
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
    let mut walker = EntryWalker::new(source.line_indent_width(body.range.start()));
    for line in UniversalNewlineIterator::with_offset(body.text, body.range.start()) {
        walker.consume(line);
    }
    walker.finish_section();
    walker.sections
}

#[cfg(test)]
mod tests {
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
    fn entry_carrying_sections_recognizes_type_bearing_entry_by_bare_name() {
        let src = "def f():\n    \"\"\"\n    Args:\n        markup (str): console markup.\n        width (Dict[str, int]): the budget.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(entry_names(&sections), vec![vec!["markup", "width"]]);
    }

    #[test]
    fn entry_carrying_sections_reports_each_head_line_colon_offset() {
        let src = "def f():\n    \"\"\"\n    Args:\n        markup (str): console markup.\n        width: the budget.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        for entry in &sections[0] {
            assert_eq!(
                s.slice(TextRange::new(entry.colon, entry.colon + TextSize::of(':'))),
                ":",
            );
        }
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
}
