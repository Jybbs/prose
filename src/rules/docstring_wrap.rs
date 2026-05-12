//! Wraps Google-style docstring prose to its configured budget.
//! Description prose wraps to `docstring_line_length`. Structured
//! sections (`Args:`, `Attributes:`, `Examples:`, `Note:`, `Raises:`,
//! `Returns:`, `Warning:`, `Yields:`) wrap to the budget that
//! `docstring_structured_policy` selects. Verbatim regions (triple-
//! backtick fences, blocks indented one step beyond the body, list
//! items and their continuations) pass through unchanged.
//! reStructuredText markup, Sphinx directives, and Numpydoc style
//! pass through unwrapped.

use std::num::NonZeroUsize;

use ruff_diagnostics::Edit;
use ruff_python_ast::StringLiteral;
use ruff_python_trivia::leading_indentation;
use textwrap::Options;

use crate::config::{Config, DocstringStructuredPolicy};
use crate::primitives::docstring::{indent_prefix, triple_quoted_body, DocstringHandler};
use crate::primitives::edit::narrowed_replacement;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

/// Google-style section names recognized by the parser.
const SECTIONS: &[&str] = &[
    "Args",
    "Attributes",
    "Examples",
    "Note",
    "Raises",
    "Returns",
    "Warning",
    "Yields",
];

pub(crate) struct DocstringWrap {
    description_width: usize,
    section_width: usize,
}

impl DocstringWrap {
    pub(crate) fn from_config(config: &Config) -> Self {
        let description_width =
            width_or_panic(config.docstring_line_length, "docstring-line-length");
        let code_width = width_or_panic(config.code_line_length, "code-line-length");
        let section_width = match config.docstring_structured_policy {
            DocstringStructuredPolicy::CodeLineLength => code_width,
            DocstringStructuredPolicy::DocstringLineLength => description_width,
        };
        Self {
            description_width,
            section_width,
        }
    }
}

impl Rule for DocstringWrap {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut rewriter = Rewriter {
            edits: Vec::new(),
            rule: self,
            source,
        };
        rewriter.walk(source);
        rewriter.edits
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(DocstringWrap))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Description,
    Section,
}

struct Rewriter<'a> {
    edits: Vec<Edit>,
    rule: &'a DocstringWrap,
    source: &'a Source,
}

impl DocstringHandler for Rewriter<'_> {
    fn handle(&mut self, lit: &StringLiteral) {
        let Some(body) = triple_quoted_body(self.source, lit) else {
            return;
        };
        if !body.text.contains('\n') {
            return;
        }
        let indent = indent_prefix(self.source, lit);
        let newline = self.source.newline_str();
        let Some(rewritten) = rewrite_body(body.text, indent, newline, self.rule) else {
            return;
        };
        self.edits
            .extend(narrowed_replacement(self.source, body.range, rewritten));
    }
}

struct Walker<'a> {
    body_indent_chars: usize,
    in_fence: bool,
    list_indent: Option<usize>,
    newline: &'a str,
    out: String,
    paragraph: Vec<String>,
    paragraph_indent: String,
    region: Region,
    rule: &'a DocstringWrap,
}

impl Walker<'_> {
    fn buffer(&mut self, indent: &str, line: &str) {
        if self.paragraph.is_empty() {
            self.paragraph_indent = indent.to_owned();
        }
        self.paragraph.push(line.to_owned());
    }

    fn consume(&mut self, line: &str) {
        let indent_str = leading_indentation(line);
        let trimmed = &line[indent_str.len()..];
        let indent_chars = indent_str.chars().count();

        if trimmed.starts_with("```") {
            self.flush_paragraph();
            self.list_indent = None;
            self.in_fence = !self.in_fence;
            self.emit_verbatim(line);
            return;
        }
        if self.in_fence {
            self.emit_verbatim(line);
            return;
        }

        if trimmed.is_empty() {
            self.flush_paragraph();
            self.list_indent = None;
            self.region = Region::Description;
            self.out.push_str(self.newline);
            return;
        }

        if let Some(list_indent) = self.list_indent {
            if indent_chars > list_indent {
                self.emit_verbatim(line);
                return;
            }
            self.list_indent = None;
        }

        if indent_chars >= self.body_indent_chars && is_list_marker(trimmed) {
            self.flush_paragraph();
            self.list_indent = Some(indent_chars);
            self.emit_verbatim(line);
            return;
        }

        if indent_chars == self.body_indent_chars && is_section_heading(trimmed) {
            self.flush_paragraph();
            self.region = Region::Section;
            self.emit_verbatim(line);
            return;
        }

        let prose_indent = match self.region {
            Region::Description => self.body_indent_chars,
            Region::Section => self.body_indent_chars + 4,
        };
        if indent_chars > prose_indent {
            self.flush_paragraph();
            self.emit_verbatim(line);
            return;
        }

        if self.region == Region::Section && indent_chars < prose_indent {
            self.flush_paragraph();
            self.region = Region::Description;
        }

        let text = trimmed.trim_end();
        match self.region {
            Region::Description => self.buffer(indent_str, text),
            Region::Section => self.emit_wrapped(indent_str, text, self.rule.section_width),
        }
    }

    fn emit_verbatim(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push_str(self.newline);
    }

    fn emit_wrapped(&mut self, indent: &str, text: &str, width: usize) {
        let opts = Options::new(width)
            .break_words(false)
            .initial_indent(indent)
            .subsequent_indent(indent);
        for piece in textwrap::wrap(text, opts) {
            self.emit_verbatim(&piece);
        }
    }

    fn flush_paragraph(&mut self) {
        if self.paragraph.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.paragraph).join(" ");
        let indent = std::mem::take(&mut self.paragraph_indent);
        self.emit_wrapped(&indent, &text, self.rule.description_width);
    }
}

fn is_list_marker(trimmed: &str) -> bool {
    if ["- ", "* ", "+ "].iter().any(|m| trimmed.starts_with(m)) {
        return true;
    }
    let after_digits = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    after_digits.len() < trimmed.len() && after_digits.starts_with(". ")
}

fn is_section_heading(trimmed: &str) -> bool {
    SECTIONS
        .iter()
        .find_map(|name| trimmed.strip_prefix(name))
        .is_some_and(|rest| rest.starts_with(':'))
}

fn rewrite_body(
    body: &str,
    body_indent: &str,
    newline: &str,
    rule: &DocstringWrap,
) -> Option<String> {
    let stripped = body.strip_prefix(newline)?;
    let (content, closer_indent) = stripped.rsplit_once(newline)?;

    let mut walker = Walker {
        body_indent_chars: body_indent.chars().count(),
        in_fence: false,
        list_indent: None,
        newline,
        out: String::with_capacity(content.len()),
        paragraph: Vec::new(),
        paragraph_indent: String::new(),
        region: Region::Description,
        rule,
    };
    for line in content.split(newline) {
        walker.consume(line);
    }
    walker.flush_paragraph();

    let mut result = String::with_capacity(body.len());
    result.push_str(newline);
    result.push_str(walker.out.trim_end_matches(newline));
    result.push_str(newline);
    result.push_str(closer_indent);
    Some(result)
}

fn width_or_panic(opt: Option<NonZeroUsize>, field: &'static str) -> usize {
    opt.unwrap_or_else(|| panic!("config field `{field}` defaults to Some"))
        .get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;
    use crate::test_support::parse;

    fn run(src: &str) -> String {
        let source = parse(src);
        let pipeline =
            Pipeline::for_rule("docstring-wrap", &Config::default()).expect("rule is registered");
        pipeline
            .run(source)
            .expect("pipeline runs")
            .0
            .text()
            .to_owned()
    }

    #[test]
    fn closing_indent_preserved_after_wrap() {
        let long = "x".repeat(80);
        let src = format!("def f():\n    \"\"\"\n    {long}\n    \"\"\"\n");
        let out = run(&src);
        assert!(out.ends_with("\n    \"\"\"\n"));
    }

    #[test]
    fn description_short_line_is_left_alone() {
        let src = "\"\"\"\nShort summary.\n\"\"\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn description_wraps_to_default_76_character_budget() {
        let src = "\"\"\"\nThis is a long description line that exceeds the seventy six character docstring budget by a margin.\n\"\"\"\n";
        let out = run(src);
        let body_lines: Vec<&str> = out.lines().filter(|l| !l.starts_with("\"\"\"")).collect();
        assert!(body_lines.iter().all(|l| l.chars().count() <= 76));
    }

    #[test]
    fn fenced_code_block_passes_through_verbatim() {
        let src =
            "\"\"\"\nSummary.\n\n```python\nx = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12\n```\n\"\"\"\n";
        assert_eq!(run(src), src);
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
    fn is_section_heading_matches_recognized_names_with_colon() {
        for name in SECTIONS {
            assert!(
                is_section_heading(&format!("{name}:")),
                "{name}: should match"
            );
        }
        assert!(is_section_heading("Returns: int"));
        assert!(!is_section_heading("Args :"));
        assert!(!is_section_heading("args:"));
        assert!(!is_section_heading("Argz:"));
        assert!(!is_section_heading("Args"));
        assert!(!is_section_heading("Arguments:"));
    }

    #[test]
    fn list_items_and_their_continuations_are_left_alone() {
        let src = "\"\"\"\nA list:\n\n- first item here that runs on with extra words and more padding text\n  continuation indented under the first item\n- second item\n\"\"\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn non_triple_quoted_string_is_left_alone() {
        let src = "def f():\n    \"summary\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn section_body_wraps_to_section_budget_under_default_policy() {
        let src = "\"\"\"\nSummary.\n\nArgs:\n    foo: a very long parameter description that should wrap at eighty eight characters because it lives inside a structured section.\n\"\"\"\n";
        let out = run(src);
        for line in out.lines() {
            assert!(line.chars().count() <= 88, "line over 88: {line:?}");
        }
    }

    #[test]
    fn singleton_docstring_is_left_alone() {
        let src = "def f():\n    \"\"\"summary\"\"\"\n";
        assert_eq!(run(src), src);
    }
}
