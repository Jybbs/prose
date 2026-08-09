//! Google-style section walking: each Title-case heading with the
//! entries beneath it, and the continuation lines attached to each
//! entry. The per-line grammar the walk dispatches on lives in
//! `grammar`.

use ruff_python_ast::StringLiteral;
use ruff_source_file::{Line, UniversalNewlineIterator};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    body::{DocstringBody, triple_quoted_body},
    grammar::{section_heading, sibling_entry_head},
    scan::{LineScan, LineScanner, ScannedLine},
};
use crate::source::Source;

/// One Google-style section, its heading read without the trailing `:`
/// and its entries in source order.
pub(crate) struct Section<'a> {
    pub(crate) entries: Vec<SectionEntry<'a>>,
    pub(crate) heading: &'a str,
}

/// One `name: description` entry inside a Google-style section. The
/// range covers the entry's head line through the last continuation
/// line attached to it, excluding the trailing newline. `colon` is the
/// source offset of the head line's separating `:`, and `type_group`
/// the source range of the parenthesized type where the head carries
/// one.
pub(crate) struct SectionEntry<'a> {
    pub(crate) colon: TextSize,
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
    pub(crate) type_group: Option<TextRange>,
}

impl Ranged for SectionEntry<'_> {
    fn range(&self) -> TextRange {
        self.range
    }
}

struct EntryWalker<'src> {
    open_entry: Option<SectionEntry<'src>>,
    open_section: Option<Section<'src>>,
    scanner: LineScanner,
    sections: Vec<Section<'src>>,
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
            self.open_section = section_heading(trimmed).map(|heading| Section {
                entries: Vec::new(),
                heading,
            });
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
            let head_start = line_start + TextSize::of(indent);
            self.open_entry = Some(SectionEntry {
                colon: head_start + TextSize::of(&trimmed[..head.colon]),
                name: head.name,
                range: TextRange::new(line_start, line_end),
                type_group: head.type_group.map(|group| {
                    TextRange::at(
                        head_start + TextSize::of(&trimmed[..group.start]),
                        TextSize::of(&trimmed[group]),
                    )
                }),
            });
            return;
        }
        self.extend_open_entry(line_end);
    }

    fn extend_open_entry(&mut self, line_end: TextSize) {
        if let Some(entry) = self.open_entry.as_mut() {
            entry.range = entry.range.cover_offset(line_end);
        }
    }

    fn finish_entry(&mut self) {
        let Some(entry) = self.open_entry.take() else {
            return;
        };
        self.open_section
            .as_mut()
            .expect("open_entry only set while open_section is Some")
            .entries
            .push(entry);
    }

    fn finish_section(&mut self) {
        self.finish_entry();
        if let Some(section) = self.open_section.take().filter(|s| !s.entries.is_empty()) {
            self.sections.push(section);
        }
    }
}

/// Walks the entry-carrying Google-style sections in `lit`'s body
/// and returns each section's heading and entries with source-relative
/// byte ranges. Returns an empty vector unless `lit` is a multi-line
/// triple-quoted docstring on its own line holding at least one
/// recognized entry inside an entry-carrying section.
pub(crate) fn entry_carrying_sections<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Section<'src>> {
    let Some(body) = triple_quoted_body(source, lit).filter(DocstringBody::is_multiline) else {
        return Vec::new();
    };
    let mut walker = EntryWalker::new(source.line_indent_width(lit.start()));
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

    fn entry_names<'a>(sections: &[Section<'a>]) -> Vec<Vec<&'a str>> {
        sections
            .iter()
            .map(|s| s.entries.iter().map(|e| e.name).collect())
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
        let value_error_slice = s.slice(sections[0].entries[0].range);
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
        let foo_slice = s.slice(sections[0].entries[0].range);
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
        let value_error_slice = s.slice(sections[0].entries[0].range);
        assert!(value_error_slice.contains(">>> sample"));
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
        for entry in &sections[0].entries {
            assert_eq!(
                s.slice(TextRange::new(entry.colon, entry.colon + TextSize::of(':'))),
                ":",
            );
        }
    }

    #[test]
    fn entry_carrying_sections_reports_each_head_line_type_group() {
        let src = "def f():\n    \"\"\"\n    Args:\n        markup (str): console markup.\n        width (Dict[str, int]): the budget.\n        plain: no type group.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        let groups: Vec<Option<&str>> = sections[0]
            .entries
            .iter()
            .map(|entry| entry.type_group.map(|range| s.slice(range)))
            .collect();
        assert_eq!(groups, [Some("(str)"), Some("(Dict[str, int])"), None]);
    }

    #[test]
    fn entry_carrying_sections_reports_the_section_heading() {
        let src = "def f():\n    \"\"\"\n    Args:\n        b: one\n\n    Other Parameters:\n        z: three\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        let headings: Vec<&str> = sections.iter().map(|section| section.heading).collect();
        assert_eq!(headings, ["Args", "Other Parameters"]);
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
