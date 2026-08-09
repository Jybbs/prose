//! Docstring entry walking: each Title-case heading with the entries
//! beneath it and the continuation lines attached to each, plus every
//! contiguous run of type-bearing heads sitting at the body indent
//! outside those sections. The per-line grammar the walk dispatches on
//! lives in `grammar`.

use ruff_python_ast::StringLiteral;
use ruff_source_file::{Line, UniversalNewlineIterator};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    body::{DocstringBody, triple_quoted_body},
    grammar::{EntryHead, section_heading, sibling_entry_head},
    scan::{LineScan, LineScanner, ScannedLine},
};
use crate::source::Source;

/// One Google-style section, its heading read without the trailing `:`
/// and its entries in source order.
pub(crate) struct Section<'a> {
    pub(crate) entries: Vec<SectionEntry<'a>>,
    pub(crate) heading: &'a str,
}

/// One `name: description` entry inside a docstring. The range covers
/// the entry's head line through the last continuation line attached to
/// it, excluding the trailing newline. `colon` is the source offset of
/// the head line's separating `:`, and `type_group` the source range of
/// the parenthesized type where the head carries one.
pub(crate) struct SectionEntry<'a> {
    pub(crate) colon: TextSize,
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
    pub(crate) type_group: Option<TextRange>,
}

impl SectionEntry<'_> {
    /// The offset of a type group written in the Google `name (type)`
    /// form, whitespace separating the name from the `(`. `None` for a
    /// head naming no type and for a `(` sitting flush against its
    /// name, which documents a call rather than a type and so opens no
    /// alignment column.
    pub(crate) fn column_anchor(&self, source: &Source) -> Option<TextSize> {
        let group = self.type_group?;
        source
            .slice(TextRange::new(self.range.start(), group.start()))
            .ends_with(char::is_whitespace)
            .then(|| group.start())
    }
}

impl Ranged for SectionEntry<'_> {
    fn range(&self) -> TextRange {
        self.range
    }
}

/// One run of entries the walk yields, carrying its heading where a
/// Title-case section opened it and `None` where a contiguous run of
/// type-bearing heads stands outside every section.
struct EntryRun<'src> {
    entries: Vec<SectionEntry<'src>>,
    heading: Option<&'src str>,
}

struct EntryWalker<'src> {
    open_entry: Option<SectionEntry<'src>>,
    open_heading: Option<&'src str>,
    open_loose: Vec<SectionEntry<'src>>,
    open_section: Option<Vec<SectionEntry<'src>>>,
    prose_open: bool,
    runs: Vec<EntryRun<'src>>,
    scanner: LineScanner,
}

impl<'src> EntryWalker<'src> {
    fn new(body_indent_chars: usize) -> Self {
        Self {
            open_entry: None,
            open_heading: None,
            open_loose: Vec::new(),
            open_section: None,
            prose_open: false,
            runs: Vec::new(),
            scanner: LineScanner::new(body_indent_chars),
        }
    }

    /// Closes the open loose run and the open description paragraph,
    /// the pair a blank line, a verbatim region, and a section heading
    /// all reset.
    fn close_prose(&mut self) {
        self.finish_loose();
        self.prose_open = false;
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
            LineScan::Blank => self.close_prose(),
            LineScan::Body => {
                self.consume_body(line_start, line_end, indent, trimmed, indent_chars);
            }
            LineScan::Fence
            | LineScan::InFence
            | LineScan::ListContinuation
            | LineScan::ListMarker
            | LineScan::Verbatim
            | LineScan::VerbatimOpen => {
                self.close_prose();
                self.extend_open_entry(line_end);
            }
        }
    }

    /// Dispatches a body line on its indent. A line at the body indent
    /// opens a section, joins the loose run, or opens a description
    /// paragraph, whereas a deeper line continues the open entry inside
    /// a section and a shallower one joins the paragraph.
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
            if let Some(heading) = section_heading(trimmed) {
                self.close_prose();
                self.open_heading = Some(heading);
                self.open_section = Some(Vec::new());
                return;
            }
            let Some(head) =
                sibling_entry_head(indent_chars, body_indent, trimmed).filter(|_| !self.prose_open)
            else {
                self.finish_loose();
                self.prose_open = true;
                return;
            };
            let entry = entry(line_start, line_end, indent, trimmed, &head);
            if entry.type_group.is_none() {
                self.finish_loose();
                return;
            }
            self.open_loose.push(entry);
            return;
        }
        if self.open_section.is_none() {
            self.finish_loose();
            self.prose_open = indent_chars < body_indent;
            return;
        }
        if let Some(head) = sibling_entry_head(
            indent_chars,
            self.scanner.section_body_indent_chars(),
            trimmed,
        ) {
            self.finish_entry();
            self.open_entry = Some(entry(line_start, line_end, indent, trimmed, &head));
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
            .push(entry);
    }

    fn finish_loose(&mut self) {
        let entries = std::mem::take(&mut self.open_loose);
        self.push_run(entries, None);
    }

    fn finish_section(&mut self) {
        self.finish_entry();
        if let Some(entries) = self.open_section.take() {
            let heading = self.open_heading.take();
            self.push_run(entries, heading);
        }
    }

    /// Records `entries` as a run, dropping an empty one so a section
    /// with no entry and a run that never opened both yield nothing.
    fn push_run(&mut self, entries: Vec<SectionEntry<'src>>, heading: Option<&'src str>) {
        if !entries.is_empty() {
            self.runs.push(EntryRun { entries, heading });
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
    walk(source, lit)
        .into_iter()
        .filter_map(|run| {
            Some(Section {
                entries: run.entries,
                heading: run.heading?,
            })
        })
        .collect()
}

/// Every run of entries `lit`'s body carries in source order, one per
/// entry-carrying Google-style section and one per contiguous run of
/// type-bearing heads at the body indent outside every section. A
/// sectionless run gathers the heads `docstring-wrap` passes through
/// verbatim, leaving out a head that reflows into the prose above it.
pub(crate) fn entry_runs<'src>(
    source: &'src Source,
    lit: &StringLiteral,
) -> Vec<Vec<SectionEntry<'src>>> {
    walk(source, lit)
        .into_iter()
        .map(|run| run.entries)
        .collect()
}

/// Builds the entry a head line opens, resolving the head's in-line
/// byte offsets against the line's start and indent.
fn entry<'src>(
    line_start: TextSize,
    line_end: TextSize,
    indent: &str,
    trimmed: &'src str,
    head: &EntryHead<'src>,
) -> SectionEntry<'src> {
    let head_start = line_start + TextSize::of(indent);
    SectionEntry {
        colon: head_start + TextSize::of(&trimmed[..head.colon]),
        name: head.name,
        range: TextRange::new(line_start, line_end),
        type_group: head.type_group.clone().map(|group| {
            TextRange::at(
                head_start + TextSize::of(&trimmed[..group.start]),
                TextSize::of(&trimmed[group]),
            )
        }),
    }
}

/// Walks `lit`'s body and returns every entry run it carries, in
/// source order. Yields nothing unless `lit` is a multi-line
/// triple-quoted docstring sitting on its own line.
fn walk<'src>(source: &'src Source, lit: &StringLiteral) -> Vec<EntryRun<'src>> {
    let Some(body) = triple_quoted_body(source, lit).filter(DocstringBody::is_multiline) else {
        return Vec::new();
    };
    let mut walker = EntryWalker::new(source.line_indent_width(lit.start()));
    for line in UniversalNewlineIterator::with_offset(body.text, body.range.start()) {
        walker.consume(line);
    }
    walker.finish_section();
    walker.finish_loose();
    walker.runs
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        primitives::docstring::body_docstring,
        testing::{first_class, first_def, parse},
    };

    fn class_docstring(source: &Source) -> &StringLiteral {
        body_docstring(&first_class(source).body).expect("class body starts with a docstring")
    }

    fn entry_names<'a>(runs: &[Vec<SectionEntry<'a>>]) -> Vec<Vec<&'a str>> {
        runs.iter()
            .map(|run| run.iter().map(|e| e.name).collect())
            .collect()
    }

    fn section_names<'a>(sections: &[Section<'a>]) -> Vec<Vec<&'a str>> {
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
        assert_eq!(
            section_names(&sections),
            vec![vec!["ValueError", "OSError"]]
        );
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
        assert_eq!(section_names(&sections), vec![vec!["foo", "bar"]]);
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
        assert_eq!(
            section_names(&sections),
            vec![vec!["ValueError", "OSError"]]
        );
        let value_error_slice = s.slice(sections[0].entries[0].range);
        assert!(value_error_slice.contains(">>> sample"));
    }

    #[test]
    fn entry_carrying_sections_groups_entries_per_section() {
        let src = "def f():\n    \"\"\"\n    Args:\n        b: one\n        a: two\n\n    Returns:\n        z: three\n        y: four\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(
            section_names(&sections),
            vec![vec!["b", "a"], vec!["z", "y"]]
        );
    }

    #[test]
    fn entry_carrying_sections_omits_a_sectionless_run() {
        let src = "class C:\n    \"\"\"\n    Args:\n        foo: one\n\n    handler (Callable): a sectionless head.\n    codec (bytes): another.\n    \"\"\"\n";
        let s = parse(src);
        let lit = class_docstring(&s);
        assert_eq!(
            section_names(&entry_carrying_sections(&s, lit)),
            vec![vec!["foo"]],
        );
    }

    #[test]
    fn entry_carrying_sections_recognizes_section_by_content_shape() {
        let src = "def f():\n    \"\"\"\n    Steps:\n        bar: second\n        alpha: first\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(section_names(&sections), vec![vec!["bar", "alpha"]]);
    }

    #[test]
    fn entry_carrying_sections_recognizes_type_bearing_entry_by_bare_name() {
        let src = "def f():\n    \"\"\"\n    Args:\n        markup (str): console markup.\n        width (Dict[str, int]): the budget.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(section_names(&sections), vec![vec!["markup", "width"]]);
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
        assert_eq!(section_names(&sections), vec![vec!["bar", "alpha"]]);
    }

    #[test]
    fn entry_carrying_sections_yields_single_entry_section() {
        let src = "def f():\n    \"\"\"\n    Returns:\n        value: the result.\n    \"\"\"\n    pass\n";
        let s = parse(src);
        let lit = first_function_docstring(&s);
        let sections = entry_carrying_sections(&s, lit);
        assert_eq!(section_names(&sections), vec![vec!["value"]]);
    }

    #[rstest]
    #[case::blank_line_splits_a_run(
        "class C:\n    \"\"\"\n    handler (Callable): one.\n\n    codec (bytes): two.\n    \"\"\"\n",
        vec![vec!["handler"], vec!["codec"]]
    )]
    #[case::verbatim_line_splits_a_run(
        "class C:\n    \"\"\"\n    handler (Callable): one.\n    ---\n    codec (bytes): two.\n    \"\"\"\n",
        vec![vec!["handler"], vec!["codec"]]
    )]
    #[case::verbatim_block_swallows_the_head_below_it(
        "class C:\n    \"\"\"\n    handler (Callable): one.\n    >>> probe()\n    codec (bytes): two.\n    \"\"\"\n",
        vec![vec!["handler"]]
    )]
    #[case::a_section_and_a_loose_run_stay_apart(
        "class C:\n    \"\"\"\n    Args:\n        foo: one\n\n    handler (Callable): a sectionless head.\n    codec (bytes): another.\n    \"\"\"\n",
        vec![vec!["foo"], vec!["handler", "codec"]]
    )]
    #[case::a_flush_paren_still_joins_its_run(
        "class C:\n    \"\"\"\n    divmod(self, other): The pair.\n    trunc(self): Truncates self.\n    \"\"\"\n",
        vec![vec!["divmod", "trunc"]]
    )]
    #[case::a_head_naming_no_type_opens_no_run(
        "class C:\n    \"\"\"\n    handler: no type group.\n    codec: none either.\n    \"\"\"\n",
        Vec::new()
    )]
    #[case::a_head_under_prose_opens_no_run(
        "class C:\n    \"\"\"\n    A summary line.\n    handler (Callable): reflowed into the prose above it.\n    \"\"\"\n",
        Vec::new()
    )]
    #[case::a_loose_run_above_a_section_stays_apart(
        "class C:\n    \"\"\"\n    handler (Callable): a sectionless head.\n\n    Args:\n        foo: one\n    \"\"\"\n",
        vec![vec!["handler"], vec!["foo"]]
    )]
    #[case::a_head_under_dedented_prose_opens_no_run(
        "class C:\n    \"\"\"\n  dedented opening prose.\n    handler (Callable): reflowed into the prose above it.\n    \"\"\"\n",
        Vec::new()
    )]
    fn entry_runs_groups_each_contiguous_run(#[case] src: &str, #[case] expected: Vec<Vec<&str>>) {
        let s = parse(src);
        let lit = class_docstring(&s);
        assert_eq!(entry_names(&entry_runs(&s, lit)), expected);
    }

    #[test]
    fn entry_runs_anchors_each_column_on_its_type_group() {
        let src = "class C:\n    \"\"\"\n    handler (Callable): one.\n    codec (bytes): two.\n    \"\"\"\n";
        let s = parse(src);
        let lit = class_docstring(&s);
        for entry in &entry_runs(&s, lit)[0] {
            let anchor = entry
                .column_anchor(&s)
                .expect("a spaced type group anchors a column");
            assert_eq!(
                s.slice(TextRange::new(anchor, anchor + TextSize::of('('))),
                "(",
            );
        }
    }
}
