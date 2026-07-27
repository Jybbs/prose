//! Wraps Google-style docstring prose to its configured budget.
//! Description prose wraps to `docstring_line_length`, Title-case-headed
//! sections to the budget `docstring_structured_policy` selects, and
//! each `name: description` entry to `docstring_line_length` with a
//! hanging indent, its head left verbatim. Fences, over-indented
//! blocks, list items, doctests, section underlines, directives, and a
//! field list opening its own block pass through unchanged, whereas an
//! interpreted-text role is prose. Description and section prose alike
//! collapse every interior whitespace run to one space.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::Ranged;
use textwrap::{Options, WordSeparator, WordSplitter};

use crate::{
    config::{Config, DocstringStructuredPolicy},
    primitives::{
        docstring::{
            DocstringBody, LineScan, LineScanner, ScannedLine, entry_head, rewrite_docstrings,
            section_heading, triple_quoted_body,
        },
        edit::narrowed_replacement,
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
        rewrite_docstrings(source, |source, lit, edits| {
            let Some(body) = triple_quoted_body(source, lit).filter(DocstringBody::is_multiline)
            else {
                return;
            };
            let newline = source.newline_str();
            let width = source.line_indent_width(lit.start());
            let Some(rewritten) = rewrite_body(body.text, width, newline, self) else {
                return;
            };
            edits.extend(narrowed_replacement(source, body.range, rewritten));
        })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[derive(Default)]
struct Paragraph<'a> {
    head: &'a str,
    initial_indent: &'a str,
    lines: Vec<&'a str>,
    subsequent_indent: Cow<'a, str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Description,
    Section,
    SectionEntry(usize),
}

struct Walker<'a> {
    newline: &'a str,
    out: String,
    paragraph: Paragraph<'a>,
    region: Region,
    rule: &'a DocstringWrap,
    scanner: LineScanner,
}

impl<'a> Walker<'a> {
    fn buffer_description(&mut self, indent: &'a str, line: &'a str) {
        if self.paragraph.lines.is_empty() {
            self.paragraph.initial_indent = indent;
            self.paragraph.subsequent_indent = Cow::Borrowed(indent);
        }
        self.paragraph.lines.push(line);
    }

    fn consume(&mut self, line: &'a str) {
        let ScannedLine {
            indent,
            indent_chars,
            scan,
            trimmed,
        } = self.scanner.scan_line(line);

        match scan {
            LineScan::Fence | LineScan::ListMarker | LineScan::VerbatimOpen => {
                self.flush_paragraph();
                self.emit_verbatim(line);
                return;
            }
            LineScan::InFence | LineScan::ListContinuation | LineScan::Verbatim => {
                self.emit_verbatim(line);
                return;
            }
            LineScan::Blank => {
                self.flush_paragraph();
                self.out.push_str(self.newline);
                return;
            }
            LineScan::Body => {}
        }

        let body_indent = self.scanner.body_indent_chars();
        if indent_chars == body_indent && section_heading(trimmed) {
            self.flush_paragraph();
            self.region = Region::Section;
            self.emit_verbatim(line);
            return;
        }

        let text = trimmed.trim_end();
        if let Region::SectionEntry(hanging_col) = self.region {
            if self.is_entry_continuation(indent_chars, text, hanging_col) {
                self.paragraph.lines.push(text);
                return;
            }
            self.flush_paragraph();
        }

        let prose_indent = match self.region {
            Region::Description => body_indent,
            Region::Section => self.scanner.section_body_indent_chars(),
            Region::SectionEntry(_) => unreachable!("entries handled above"),
        };
        if indent_chars > prose_indent {
            self.flush_paragraph();
            self.emit_verbatim(line);
            return;
        }

        if self.region == Region::Section && indent_chars < prose_indent {
            self.region = Region::Description;
        }

        match self.region {
            Region::Description => self.buffer_description(indent, text),
            Region::Section => {
                if let Some((_, desc_start)) = entry_head(text) {
                    self.start_entry(indent, indent_chars, text, desc_start);
                    return;
                }
                self.emit_wrapped(indent, indent, &collapsed([text]), self.rule.section_width);
            }
            Region::SectionEntry(_) => unreachable!("entries handled above"),
        }
    }

    fn emit_verbatim(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push_str(self.newline);
    }

    fn emit_wrapped(&mut self, initial: &str, subsequent: &str, text: &str, width: usize) {
        // AsciiSpace and NoHyphenation keep a slash- or hyphen-bearing token
        // atomic, so an over-budget URL or path overflows instead of splitting.
        let opts = Options::new(width)
            .break_words(false)
            .initial_indent(initial)
            .subsequent_indent(subsequent)
            .word_separator(WordSeparator::AsciiSpace)
            .word_splitter(WordSplitter::NoHyphenation);
        for piece in textwrap::wrap(text, opts) {
            self.emit_verbatim(&piece);
        }
    }

    fn flush_paragraph(&mut self) {
        if !self.paragraph.lines.is_empty() {
            let Paragraph {
                head,
                initial_indent,
                lines,
                subsequent_indent,
            } = std::mem::take(&mut self.paragraph);
            let text = [head, &collapsed(lines)].concat();
            self.emit_wrapped(
                initial_indent,
                &subsequent_indent,
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
            || (indent_chars == self.scanner.section_body_indent_chars()
                && entry_head(trimmed).is_none())
    }

    fn start_entry(
        &mut self,
        indent_str: &'a str,
        indent_chars: usize,
        text: &'a str,
        desc_start: usize,
    ) {
        let (head, description) = text.split_at(desc_start);
        let hanging_col = indent_chars + head.chars().count();
        self.paragraph.head = head;
        self.paragraph.initial_indent = indent_str;
        self.paragraph.subsequent_indent = " ".repeat(hanging_col).into();
        self.paragraph.lines.push(description);
        self.region = Region::SectionEntry(hanging_col);
    }
}

/// Joins `lines` into one prose run, collapsing every whitespace run
/// between words to a single space.
fn collapsed<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    lines.into_iter().flat_map(str::split_whitespace).join(" ")
}

fn rewrite_body(
    body: &str,
    body_indent_chars: usize,
    newline: &str,
    rule: &DocstringWrap,
) -> Option<String> {
    let (content, closer_indent) = body.strip_prefix(newline)?.rsplit_once(newline)?;

    let mut walker = Walker {
        newline,
        out: String::with_capacity(content.len()),
        paragraph: Paragraph::default(),
        region: Region::Description,
        rule,
        scanner: LineScanner::new(body_indent_chars),
    };
    for line in content.split(newline) {
        walker.consume(line);
    }
    walker.flush_paragraph();

    let wrapped = walker.out.trim_end_matches(newline);
    Some([newline, wrapped, newline, closer_indent].concat())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::run_rule;

    fn run(src: &str) -> String {
        run_rule("docstring-wrap", src)
    }

    #[test]
    fn aligned_entry_gap_survives_the_rewrap() {
        let src = "def f():\n    \"\"\"\n    Args:\n        host     : A descriptive parameter that runs on long enough to force a wrap onto a second line.\n        encoding : Short.\n    \"\"\"\n    pass\n";
        assert!(
            run(src).contains("host     : A descriptive"),
            "the column `align-colons` set was collapsed by the wrap",
        );
    }

    #[test]
    fn closing_indent_preserved_after_wrap() {
        let long = "x".repeat(80);
        let src = format!("def f():\n    \"\"\"\n    {long}\n    \"\"\"\n");
        let out = run(&src);
        assert!(out.ends_with("\n    \"\"\"\n"));
    }

    #[test]
    fn collapsed_joins_lines_and_squeezes_interior_runs() {
        assert_eq!(collapsed(["one  two", "three\tfour"]), "one two three four");
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

    #[rstest]
    fn over_budget_token_with_embedded_break_overflows_unbroken(
        #[values(
            "https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Security-Policy",
            "config-loader/runtime-overrides/per-file-ancestors/resolution-and-precedence-order"
        )]
        token: &str,
    ) {
        let src = format!(
            "\"\"\"\nThe canonical reference value lives at {token} for callers here.\n\"\"\"\n"
        );
        assert!(
            run(&src).contains(token),
            "atomic token was split at an embedded `/` or `-`"
        );
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

    #[test]
    fn type_bearing_entry_continuation_hangs_under_description_column() {
        let src = "\"\"\"\nArgs:\n    markup (str): A string containing console markup that will overflow the line budget for sure yes.\n\"\"\"\n";
        let out = run(src);
        let continuation = out
            .lines()
            .skip_while(|l| !l.contains("markup (str):"))
            .nth(1)
            .expect("continuation line follows the wrapped entry head");
        let indent = continuation.len() - continuation.trim_start().len();
        assert_eq!(
            indent, 18,
            "continuation hangs under the description column"
        );
    }
}
