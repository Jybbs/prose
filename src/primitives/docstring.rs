//! Shared walker for PEP 257 docstring statements, the first
//! body-statement of the module, each class, and each function that
//! holds a string literal as its first expression statement.
//! Implementors of [`DocstringHandler`] receive every such docstring
//! literal in source order via the trait's `walk` method. Implicitly
//! concatenated docstring expressions are skipped. The section
//! helpers `section_heading`, `entry_description_col`, and
//! `entry_carrying_sections` parse a docstring body's Title-case-headed
//! sections for consumers that walk text rather than the AST,
//! recognizing entry-carrying sections by content shape rather than
//! against a closed name list.

use std::sync::LazyLock;

use regex_lite::Regex;
use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
use ruff_python_ast::{ExprStringLiteral, Stmt, StringFlags, StringLiteral};
use ruff_python_trivia::{has_leading_content, leading_indentation};
use ruff_source_file::{Line, LineRanges, UniversalNewlineIterator};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::source::Source;

static ENTRY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\w[\w.]*\s*:\s+\S").expect("static pattern compiles"));

static SECTION_HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Z][A-Za-z]*( [A-Z][A-Za-z]*)*:").expect("static pattern compiles")
});

/// Body slice between a triple-quoted docstring's opener and closer,
/// paired with the source range that slice covers.
pub(crate) struct DocstringBody<'a> {
    pub(crate) range: TextRange,
    pub(crate) text: &'a str,
}

/// Receiver for the docstring walker. Implementors handle each
/// docstring `StringLiteral` reached in source order. Call `walk`
/// to drive the receiver across `source`'s module body.
pub(crate) trait DocstringHandler {
    fn handle(&mut self, lit: &StringLiteral);

    fn walk(&mut self, source: &Source)
    where
        Self: Sized,
    {
        let mut visitor = Visitor { handler: self };
        let body = &source.ast().body;
        visitor.consider(body);
        visitor.visit_body(body);
    }
}

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
    body_indent_chars: usize,
    in_fence: bool,
    list_indent: Option<usize>,
    open_entry: Option<SectionEntry<'src>>,
    open_section: Option<Vec<SectionEntry<'src>>>,
    sections: Vec<Vec<SectionEntry<'src>>>,
}

impl<'src> EntryWalker<'src> {
    fn new(body_indent_chars: usize) -> Self {
        Self {
            body_indent_chars,
            in_fence: false,
            list_indent: None,
            open_entry: None,
            open_section: None,
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

        if trimmed.starts_with("```") {
            self.in_fence = !self.in_fence;
            self.extend_open_entry(line_end);
            self.list_indent = None;
            return;
        }
        if self.in_fence {
            self.extend_open_entry(line_end);
            return;
        }
        if trimmed.is_empty() {
            self.list_indent = None;
            return;
        }
        if let Some(marker) = self.list_indent {
            if indent_chars > marker {
                self.extend_open_entry(line_end);
                return;
            }
            self.list_indent = None;
        }
        if indent_chars >= self.body_indent_chars && is_list_marker(trimmed) {
            self.list_indent = Some(indent_chars);
            self.extend_open_entry(line_end);
            return;
        }
        if indent_chars == self.body_indent_chars {
            self.finish_section();
            if section_heading(trimmed) {
                self.open_section = Some(Vec::new());
            }
            return;
        }
        if self.open_section.is_none() {
            return;
        }
        if indent_chars == self.body_indent_chars + 4 && entry_description_col(trimmed).is_some() {
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

struct Visitor<'a, H: DocstringHandler> {
    handler: &'a mut H,
}

impl<H: DocstringHandler> Visitor<'_, H> {
    fn consider(&mut self, body: &[Stmt]) {
        let docstring = body
            .first()
            .and_then(Stmt::as_expr_stmt)
            .and_then(|e| e.value.as_string_literal_expr())
            .and_then(ExprStringLiteral::as_single_part_string);
        if let Some(lit) = docstring {
            self.handler.handle(lit);
        }
    }
}

impl<'a, H: DocstringHandler> StatementVisitor<'a> for Visitor<'_, H> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(c) => self.consider(&c.body),
            Stmt::FunctionDef(f) => self.consider(&f.body),
            _ => {}
        }
        walk_stmt(self, stmt);
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
    let Some(body) = triple_quoted_body(source, lit).filter(|b| b.text.contains('\n')) else {
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

/// Returns the line indent prefix of the docstring at `lit.start()`,
/// preserving the source's mix of tabs and spaces verbatim.
pub(crate) fn indent_prefix<'a>(source: &'a Source, lit: &StringLiteral) -> &'a str {
    leading_indentation(source.text().line_str(lit.start()))
}

/// True when `trimmed` opens with a Markdown list marker (`-`, `*`,
/// or `+` followed by a space) or a numeric marker (one or more
/// digits followed by `. `). Used by docstring walkers to recognize
/// verbatim-passthrough list items.
pub(crate) fn is_list_marker(trimmed: &str) -> bool {
    if trimmed
        .strip_prefix(['-', '*', '+'])
        .is_some_and(|rest| rest.starts_with(' '))
    {
        return true;
    }
    let after_digits = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    after_digits.len() < trimmed.len() && after_digits.starts_with(". ")
}

/// Walks every docstring in `source` and collects the edits produced
/// by `f` against each. The closure receives `source`, the docstring
/// literal, and the running edit buffer. Returns the accumulated edits.
pub(crate) fn rewrite_docstrings<F>(source: &Source, f: F) -> Vec<Edit>
where
    F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
{
    struct Collector<'a, F> {
        edits: Vec<Edit>,
        f: F,
        source: &'a Source,
    }

    impl<F> DocstringHandler for Collector<'_, F>
    where
        F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
    {
        fn handle(&mut self, lit: &StringLiteral) {
            (self.f)(self.source, lit, &mut self.edits);
        }
    }

    let mut collector = Collector {
        edits: Vec::new(),
        f,
        source,
    };
    collector.walk(source);
    collector.edits
}

/// True when `trimmed` opens with a Title-case word or multi-word
/// run with every word capitalized, immediately followed by `:`.
/// Trailing content after the `:` is permitted.
pub(crate) fn section_heading(trimmed: &str) -> bool {
    SECTION_HEADING.is_match(trimmed)
}

/// Returns the body slice and source range when `lit` is triple-quoted
/// and sits at the start of its own line. Returns `None` for
/// non-triple-quoted literals and inline `def f(): """..."""` shapes.
pub(crate) fn triple_quoted_body<'a>(
    source: &'a Source,
    lit: &StringLiteral,
) -> Option<DocstringBody<'a>> {
    if !lit.flags.is_triple_quoted() || has_leading_content(lit.start(), source.text()) {
        return None;
    }
    let range = TextRange::new(
        lit.start() + lit.flags.opener_len(),
        lit.end() - lit.flags.closer_len(),
    );
    Some(DocstringBody {
        range,
        text: source.slice(range),
    })
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
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;
    use crate::test_support::parse;

    #[derive(Default)]
    struct Probe<'a> {
        source: Option<&'a Source>,
        values: Vec<String>,
        bodies: Vec<String>,
        indents: Vec<String>,
    }

    impl Probe<'_> {
        fn run(source: &Source) -> Vec<String> {
            let mut probe = Probe::default();
            probe.walk(source);
            probe.values
        }
    }

    impl DocstringHandler for Probe<'_> {
        fn handle(&mut self, lit: &StringLiteral) {
            self.values.push(lit.value.to_string());
            if let Some(source) = self.source {
                self.indents.push(indent_prefix(source, lit).to_owned());
                self.bodies
                    .extend(triple_quoted_body(source, lit).map(|b| b.text.to_owned()));
            }
        }
    }

    fn entry_names<'a>(sections: &[Vec<SectionEntry<'a>>]) -> Vec<Vec<&'a str>> {
        sections
            .iter()
            .map(|s| s.iter().map(|e| e.name).collect())
            .collect()
    }

    fn first_function_docstring(source: &Source) -> &StringLiteral {
        let func = source.ast().body[0]
            .as_function_def_stmt()
            .expect("first stmt is a def");
        func.body[0]
            .as_expr_stmt()
            .and_then(|e| e.value.as_string_literal_expr())
            .and_then(ExprStringLiteral::as_single_part_string)
            .expect("function body starts with a docstring literal")
    }

    fn probe_with_source(source: &Source) -> Probe<'_> {
        let mut probe = Probe {
            source: Some(source),
            ..Probe::default()
        };
        probe.walk(source);
        probe
    }

    #[test]
    fn collects_class_function_and_method_docstrings_in_source_order() {
        let s = parse(
            "\"\"\"M\"\"\"\nclass C:\n    \"\"\"C\"\"\"\n    def m(self):\n        \"\"\"m\"\"\"\n        pass\n",
        );
        assert_eq!(Probe::run(&s), ["M", "C", "m"]);
    }

    #[test]
    fn collects_nested_function_docstrings() {
        let s = parse(
            "def outer():\n    \"\"\"o\"\"\"\n    def inner():\n        \"\"\"i\"\"\"\n        pass\n",
        );
        assert_eq!(Probe::run(&s), ["o", "i"]);
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

    #[test]
    fn indent_prefix_preserves_source_indent_characters() {
        let s = parse("class C:\n\t\"\"\"doc\"\"\"\n\tpass\n");
        let probe = probe_with_source(&s);
        assert_eq!(probe.indents, ["\t"]);
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
    fn returns_empty_for_module_with_no_docstrings() {
        let s = parse("x = 1\ndef f():\n    return 1\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn rewrite_docstrings_collects_edits_pushed_by_closure_per_docstring() {
        let s = parse("\"\"\"M\"\"\"\ndef f():\n    \"\"\"f\"\"\"\n    pass\n");
        let edits = rewrite_docstrings(&s, |_, lit, edits| {
            edits.push(Edit::range_deletion(lit.range()));
        });
        assert_eq!(edits.len(), 2);
        assert!(edits.windows(2).all(|w| w[0].start() < w[1].start()));
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

    #[test]
    fn skips_implicitly_concatenated_docstring_expressions() {
        let s = parse("\"\"\"a\"\"\" \"\"\"b\"\"\"\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn skips_string_expression_that_is_not_first_statement() {
        let s = parse("x = 1\n\"not a docstring\"\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn triple_quoted_body_extracts_inner_body_text() {
        let s = parse("'''hello'''\n");
        let probe = probe_with_source(&s);
        assert_eq!(probe.bodies, ["hello"]);
    }

    #[test]
    fn triple_quoted_body_rejects_inline_with_def() {
        let s = parse("def f(): \"\"\"doc\"\"\"\n");
        let probe = probe_with_source(&s);
        assert!(probe.bodies.is_empty());
    }

    #[test]
    fn triple_quoted_body_rejects_non_triple_quoted_literal() {
        let s = parse("\"hello\"\n");
        let probe = probe_with_source(&s);
        assert!(probe.bodies.is_empty());
    }
}
