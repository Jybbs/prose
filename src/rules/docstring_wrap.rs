//! Wraps Google-style docstring prose to its configured budget.
//! Description prose wraps to `docstring_line_length`. Structured
//! sections (`Args:`, `Attributes:`, `Examples:`, `Note:`, `Raises:`,
//! `Returns:`, `Warning:`, `Yields:`) wrap to the budget that
//! `docstring_structured_policy` selects. Entry-carrying sections
//! (`Args:`, `Attributes:`, `Raises:`, `Returns:`, `Yields:`) wrap
//! `name: description` entries to `docstring_line_length` with a
//! hanging indent at the description's start column. Verbatim regions
//! (triple-backtick fences, blocks indented one step beyond the body,
//! list items and their continuations) pass through unchanged.
//! reStructuredText markup, Sphinx directives, and Numpydoc style
//! pass through unwrapped.

use ruff_diagnostics::Edit;
use ruff_python_trivia::leading_indentation;
use textwrap::Options;

use crate::{
    config::{Config, DocstringStructuredPolicy},
    primitives::{
        docstring::{
            entry_description_col, indent_prefix, is_list_marker, rewrite_docstrings,
            section_heading, triple_quoted_body,
        },
        edit::{narrowed_replacement, singleton_groups},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct DocstringWrap {
    description_width: usize,
    section_width: usize,
}

impl DocstringWrap {
    pub(crate) fn from_config(config: &Config) -> Self {
        let description_width = config.docstring_width();
        let section_width = match config.docstring_structured_policy {
            DocstringStructuredPolicy::CodeLineLength => config.code_width(),
            DocstringStructuredPolicy::DocstringLineLength => description_width,
        };
        Self {
            description_width,
            section_width,
        }
    }
}

impl Rule for DocstringWrap {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(rewrite_docstrings(source, |source, lit, edits| {
            let Some(body) = triple_quoted_body(source, lit).filter(|b| b.text.contains('\n'))
            else {
                return;
            };
            let indent = indent_prefix(source, lit);
            let newline = source.newline_str();
            let Some(rewritten) = rewrite_body(body.text, indent, newline, self) else {
                return;
            };
            edits.extend(narrowed_replacement(source, body.range, rewritten));
        }))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[derive(Default)]
struct Paragraph {
    initial_indent: String,
    lines: Vec<String>,
    subsequent_indent: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Description,
    Section,
    SectionEntry(usize),
}

struct Walker<'a> {
    body_indent_chars: usize,
    in_fence: bool,
    list_indent: Option<usize>,
    newline: &'a str,
    out: String,
    paragraph: Paragraph,
    region: Region,
    rule: &'a DocstringWrap,
}

impl Walker<'_> {
    fn buffer_description(&mut self, indent: &str, line: &str) {
        if self.paragraph.lines.is_empty() {
            self.paragraph.initial_indent = indent.to_owned();
            self.paragraph.subsequent_indent = indent.to_owned();
        }
        self.paragraph.lines.push(line.to_owned());
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

        if indent_chars == self.body_indent_chars && section_heading(trimmed) {
            self.flush_paragraph();
            self.region = Region::Section;
            self.emit_verbatim(line);
            return;
        }

        let text = trimmed.trim_end();
        if let Region::SectionEntry(hanging_col) = self.region {
            if self.is_entry_continuation(indent_chars, text, hanging_col) {
                self.paragraph.lines.push(text.to_owned());
                return;
            }
            self.flush_paragraph();
        }

        let prose_indent = match self.region {
            Region::Description => self.body_indent_chars,
            Region::Section => self.body_indent_chars + 4,
            Region::SectionEntry(_) => unreachable!("entries handled above"),
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

        match self.region {
            Region::Description => self.buffer_description(indent_str, text),
            Region::Section => {
                if let Some(desc_col) = entry_description_col(text) {
                    self.start_entry(indent_str, indent_chars, text, desc_col);
                    return;
                }
                self.emit_wrapped(indent_str, indent_str, text, self.rule.section_width);
            }
            Region::SectionEntry(_) => unreachable!("entries handled above"),
        }
    }

    fn emit_verbatim(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push_str(self.newline);
    }

    fn emit_wrapped(&mut self, initial: &str, subsequent: &str, text: &str, width: usize) {
        let opts = Options::new(width)
            .break_words(false)
            .initial_indent(initial)
            .subsequent_indent(subsequent);
        for piece in textwrap::wrap(text, opts) {
            self.emit_verbatim(&piece);
        }
    }

    fn flush_paragraph(&mut self) {
        if !self.paragraph.lines.is_empty() {
            let para = std::mem::take(&mut self.paragraph);
            let text = para.lines.join(" ");
            self.emit_wrapped(
                &para.initial_indent,
                &para.subsequent_indent,
                &text,
                self.rule.description_width,
            );
        }
        if matches!(self.region, Region::SectionEntry(_)) {
            self.region = Region::Section;
        }
    }

    fn is_entry_continuation(
        &self,
        indent_chars: usize,
        trimmed: &str,
        hanging_col: usize,
    ) -> bool {
        indent_chars == hanging_col
            || (indent_chars == self.body_indent_chars + 4
                && entry_description_col(trimmed).is_none())
    }

    fn start_entry(&mut self, indent_str: &str, indent_chars: usize, text: &str, desc_col: usize) {
        let hanging_col = indent_chars + desc_col;
        self.paragraph.initial_indent = indent_str.to_owned();
        self.paragraph.subsequent_indent = " ".repeat(hanging_col);
        self.paragraph.lines.push(text.to_owned());
        self.region = Region::SectionEntry(hanging_col);
    }
}

fn rewrite_body(
    body: &str,
    body_indent: &str,
    newline: &str,
    rule: &DocstringWrap,
) -> Option<String> {
    let (content, closer_indent) = body.strip_prefix(newline)?.rsplit_once(newline)?;

    let mut walker = Walker {
        body_indent_chars: body_indent.chars().count(),
        in_fence: false,
        list_indent: None,
        newline,
        out: String::with_capacity(content.len()),
        paragraph: Paragraph::default(),
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
        assert!(
            out.lines()
                .filter(|l| !l.starts_with("\"\"\""))
                .all(|l| l.chars().count() <= 76)
        );
    }

    #[test]
    fn fenced_code_block_passes_through_verbatim() {
        let src = "\"\"\"\nSummary.\n\n```python\nx = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12\n```\n\"\"\"\n";
        assert_eq!(run(src), src);
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
    fn section_body_entry_wraps_at_hanging_column_under_default_policy() {
        let src = "\"\"\"\nSummary.\n\nArgs:\n    foo: a very long parameter description that should wrap at seventy six characters because it lives inside an entry-carrying section.\n\"\"\"\n";
        let out = run(src);
        for line in out.lines() {
            assert!(line.chars().count() <= 76, "line over 76: {line:?}");
        }
    }

    #[test]
    fn singleton_docstring_is_left_alone() {
        let src = "def f():\n    \"\"\"summary\"\"\"\n";
        assert_eq!(run(src), src);
    }
}
