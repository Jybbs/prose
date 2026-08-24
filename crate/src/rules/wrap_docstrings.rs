//! Wraps Google-style docstring prose to its configured budget.
//! Description prose wraps to `docstring_line_length`, Title-case-headed
//! sections to the budget `docstring_structured_policy` selects, and
//! each `name: description` entry to `docstring_line_length` with a
//! hanging indent, its head left verbatim and every later line from the
//! section body indent onward that opens no entry of its own gathered
//! into it. Every region [`LineScan`] marks verbatim passes through
//! unchanged, as does a `name (type):` field header opening a paragraph
//! outside any section, whereas one sitting under prose reflows into the
//! paragraph above it. Description and section prose alike collapse
//! every interior whitespace run to one space, and a backslash
//! continuing a line of non-raw prose resolves into that join rather
//! than reaching the output as a word, whereas a continuation inside a
//! passthrough region travels with the region untouched.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextRange, TextSize};
use textwrap::{Options, WordSeparator, WordSplitter, WrapAlgorithm, core::Word};

use crate::{
    config::{Config, DocstringStructuredPolicy},
    primitives::{
        docstring::{
            DocstringBody, LineScan, LineScanner, ScannedLine, opens_structure, rewrite_docstrings,
            section_heading, sibling_entry_head, triple_quoted_body, typed_entry_head,
        },
        edit::narrowed_replacement,
        padding,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct WrapDocstrings {
    description_width: usize,
    section_width: usize,
    stranding: padding::Stranding,
}

impl WrapDocstrings {
    pub(crate) const MESSAGE: &'static str = "wrap docstring prose to the configured budget";

    pub(crate) fn from_config(config: &Config) -> Self {
        let description_width = config.docstring_width();
        let section_width = match config.docstring_structured_policy {
            DocstringStructuredPolicy::CodeLineLength => config.code_width(),
            DocstringStructuredPolicy::DocstringLineLength => description_width,
        };
        Self {
            description_width,
            section_width,
            stranding: config.stranded_padding(),
        }
    }
}

impl Rule for WrapDocstrings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        rewrite_docstrings(source, |source, lit, edits| {
            let Some(body) = triple_quoted_body(source, lit).filter(DocstringBody::is_multiline)
            else {
                return;
            };
            let newline = source.newline_str();
            let indent_chars = source.line_indent_width(lit.start());
            let padding = source.stranded_padding(self.stranding);
            let Some(rewritten) =
                rewrite_body(&body, indent_chars, newline, self, source, &padding)
            else {
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
    head_slack: usize,
    initial_indent: &'a str,
    lines: Vec<&'a str>,
    subsequent_indent: Cow<'a, str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Description,
    Section,
    SectionEntry,
}

struct Walker<'a> {
    content_start: TextSize,
    newline: &'a str,
    out: String,
    padding: &'a [Edit],
    paragraph: Paragraph<'a>,
    raw: bool,
    region: Region,
    rule: &'a WrapDocstrings,
    scanner: LineScanner,
    source: &'a Source,
}

impl<'a> Walker<'a> {
    fn buffer_description(&mut self, indent: &'a str, text: &'a str) {
        if self.paragraph.lines.is_empty() {
            self.paragraph.initial_indent = indent;
            self.paragraph.subsequent_indent = Cow::Borrowed(indent);
        }
        self.paragraph.lines.push(text);
    }

    fn consume(&mut self, offset: TextSize, line: &'a str) {
        let ScannedLine {
            indent,
            indent_chars,
            scan,
            trimmed,
        } = self.scanner.scan_line(line);

        match scan {
            LineScan::Fence | LineScan::ListMarker | LineScan::VerbatimOpen => {
                self.flush_verbatim(line);
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
        if indent_chars == body_indent && section_heading(trimmed).is_some() {
            self.flush_verbatim(line);
            self.region = Region::Section;
            return;
        }

        let text = without_continuation(trimmed, self.raw).trim_end();
        if self.region == Region::SectionEntry {
            if self.is_entry_continuation(indent_chars, text) {
                self.paragraph.lines.push(text);
                return;
            }
            self.flush_paragraph();
        }

        let prose_indent = match self.region {
            Region::Description => body_indent,
            Region::Section => self.scanner.section_body_indent_chars(),
            Region::SectionEntry => unreachable!("entries handled above"),
        };
        if indent_chars > prose_indent {
            self.flush_verbatim(line);
            return;
        }

        if self.region == Region::Section && indent_chars < prose_indent {
            self.region = Region::Description;
        }

        match self.region {
            Region::Description if self.paragraph.lines.is_empty() && typed_entry_head(text) => {
                self.flush_verbatim(line);
            }
            Region::Description => self.buffer_description(indent, text),
            Region::Section => {
                if let Some(head) = sibling_entry_head(indent_chars, prose_indent, text) {
                    self.start_entry(indent, indent_chars, text, head.desc_start, offset);
                    return;
                }
                // Section prose wraps one line at a time with no
                // paragraph rejoin, so it takes the first-fit algorithm,
                // whose maximal lines re-wrap to themselves.
                let opts = wrap_options(self.rule.section_width, indent, indent)
                    .wrap_algorithm(WrapAlgorithm::FirstFit);
                for piece in textwrap::wrap(&collapsed([text]), opts) {
                    self.emit_verbatim(&piece);
                }
            }
            Region::SectionEntry => unreachable!("entries handled above"),
        }
    }

    fn emit_verbatim(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push_str(self.newline);
    }

    fn emit_wrapped(&mut self, initial: &str, subsequent: &str, text: &str, width: usize) {
        for piece in textwrap::wrap(text, wrap_options(width, initial, subsequent)) {
            self.emit_verbatim(&piece);
        }
    }

    fn flush_paragraph(&mut self) {
        if !self.paragraph.lines.is_empty() {
            let Paragraph {
                head,
                head_slack,
                initial_indent,
                lines,
                subsequent_indent,
            } = std::mem::take(&mut self.paragraph);
            // The head is a fixed prefix rather than wrappable text, so
            // it rides the initial indent, which `textwrap` never breaks
            // inside and never emits a row without a word after.
            let opening = [initial_indent, head].concat();
            let text = collapsed(lines);
            if head_slack == 0 {
                self.emit_wrapped(
                    &opening,
                    &subsequent_indent,
                    &text,
                    self.rule.description_width,
                );
            } else {
                // The rows break at the width the padding rule settles
                // the head to, the head itself emitted as written with
                // its padding left to that rule's edit.
                let measured = " ".repeat(opening.chars().count().saturating_sub(head_slack));
                let mut pieces = textwrap::wrap(
                    &text,
                    wrap_options(self.rule.description_width, &measured, &subsequent_indent),
                )
                .into_iter();
                if let Some(first) = pieces.next() {
                    self.emit_verbatim(&[opening.as_str(), &first[measured.len()..]].concat());
                }
                for piece in pieces {
                    self.emit_verbatim(&piece);
                }
            }
        }
        if self.region == Region::SectionEntry {
            self.region = Region::Section;
        }
    }

    fn flush_verbatim(&mut self, line: &str) {
        self.flush_paragraph();
        self.emit_verbatim(line);
    }

    /// True when `trimmed` continues the open entry's description,
    /// sitting from the section body indent onward and opening no
    /// sibling entry there.
    fn is_entry_continuation(&self, indent_chars: usize, trimmed: &str) -> bool {
        let section_body = self.scanner.section_body_indent_chars();
        indent_chars >= section_body
            && sibling_entry_head(indent_chars, section_body, trimmed).is_none()
    }

    fn start_entry(
        &mut self,
        indent_str: &'a str,
        indent_chars: usize,
        text: &'a str,
        desc_start: usize,
        line_offset: TextSize,
    ) {
        let (head, description) = text.split_at(desc_start);
        // The hang column reads the head at the width the padding rule
        // settles it to.
        let start = self.content_start + line_offset + TextSize::of(indent_str);
        let range = TextRange::at(start, TextSize::of(head));
        let slack = padding::slack(self.source, self.padding, range)
            .max(0)
            .cast_unsigned();
        self.paragraph.head = head;
        self.paragraph.head_slack = slack;
        self.paragraph.initial_indent = indent_str;
        self.paragraph.subsequent_indent = " "
            .repeat((indent_chars + head.chars().count()).saturating_sub(slack))
            .into();
        self.paragraph.lines.push(description);
        self.region = Region::SectionEntry;
    }
}

/// Joins `lines` into one prose run, collapsing every whitespace run
/// between words to a single space.
fn collapsed<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    lines.into_iter().flat_map(str::split_whitespace).join(" ")
}

/// Splits `line` on ASCII spaces, dropping every break opportunity
/// whose remainder opens a verbatim structure. A row head reading as a
/// list marker, a section heading, an entry head, or any other
/// structure would be parsed as that structure on the next pass, so the
/// break folds back into the word before it and the run stays on one
/// row.
fn prose_words(line: &str) -> Box<dyn Iterator<Item = Word<'_>> + '_> {
    let mut starts = Vec::new();
    let mut cursor = 0;
    for word in WordSeparator::AsciiSpace.find_words(line) {
        if starts.is_empty() || !opens_structure(&line[cursor..]) {
            starts.push(cursor);
        }
        cursor += word.word.len() + word.whitespace.len();
    }
    starts.push(line.len());
    Box::new(
        starts
            .into_iter()
            .tuple_windows()
            .map(|(start, end)| Word::from(&line[start..end])),
    )
}

fn rewrite_body<'a>(
    body: &DocstringBody<'a>,
    body_indent_chars: usize,
    newline: &'a str,
    rule: &'a WrapDocstrings,
    source: &'a Source,
    padding: &'a [Edit],
) -> Option<String> {
    let (content, closer_indent) = body.text.strip_prefix(newline)?.rsplit_once(newline)?;
    let lines = spliced_continuations(content, newline, body.raw);

    let mut walker = Walker {
        content_start: body.range.start() + TextSize::of(newline),
        newline,
        out: String::with_capacity(content.len()),
        padding,
        paragraph: Paragraph::default(),
        raw: body.raw,
        region: Region::Description,
        rule,
        scanner: LineScanner::new(body_indent_chars),
        source,
    };
    for (offset, line) in &lines {
        walker.consume(*offset, line);
    }
    walker.flush_paragraph();

    let wrapped = walker.out.trim_end_matches(newline);
    Some([newline, wrapped, newline, closer_indent].concat())
}

/// Splits `content` on `newline`, merging a continued line with the one
/// below it when neither side of the dropped backslash carries
/// whitespace, the one join the paragraph collapse cannot reproduce.
/// Every other line passes through split as written, leaving a
/// continuation inside a passthrough region byte-identical. Each line
/// is paired with the byte offset of its first physical line within
/// `content`.
fn spliced_continuations<'a>(
    content: &'a str,
    newline: &str,
    raw: bool,
) -> Vec<(TextSize, Cow<'a, str>)> {
    let mut lines: Vec<(TextSize, Cow<'a, str>)> = Vec::new();
    let mut physical = content.split(newline).peekable();
    let mut splicing = false;
    let mut offset = TextSize::default();
    while let Some(line) = physical.next() {
        let start = offset;
        offset += TextSize::of(line) + TextSize::of(newline);
        let head = without_continuation(line, raw);
        let tight = head.len() < line.len()
            && !head.ends_with(char::is_whitespace)
            && physical
                .peek()
                .is_some_and(|next| !next.starts_with(char::is_whitespace));
        let text = if tight { head } else { line };
        match lines.last_mut().filter(|_| splicing) {
            Some((_, last)) => last.to_mut().push_str(text),
            None => lines.push((start, Cow::Borrowed(text))),
        }
        splicing = tight;
    }
    lines
}

/// Drops the trailing backslash of a line continuation, leaving the join
/// to the paragraph collapse, which reads the whitespace on either side
/// of it as the separator. An odd run of trailing backslashes closes on
/// a continuation and an even run closes on an escaped backslash, and a
/// raw docstring holds no continuations at all.
fn without_continuation(line: &str, raw: bool) -> &str {
    let backslashes = line.len() - line.trim_end_matches('\\').len();
    if raw || backslashes.is_multiple_of(2) {
        return line;
    }
    &line[..line.len() - 1]
}

/// The wrap options every emission shares. The custom separator keeps a
/// slash- or hyphen-bearing token atomic alongside `NoHyphenation`, so
/// an over-budget URL or path overflows instead of splitting.
fn wrap_options<'o>(width: usize, initial: &'o str, subsequent: &'o str) -> Options<'o> {
    Options::new(width)
        .break_words(false)
        .initial_indent(initial)
        .subsequent_indent(subsequent)
        .word_separator(WordSeparator::Custom(prose_words))
        .word_splitter(WordSplitter::NoHyphenation)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::run_rule;

    fn run(src: &str) -> String {
        run_rule("wrap-docstrings", src)
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
    fn entry_continuation_below_the_hanging_column_rejoins_the_description() {
        let src = "\"\"\"\nArgs:\n    name    : A descriptive parameter whose continuation was left under a narrower colon column than this entry now carries.\n      stranded at the older column.\n\"\"\"\n";
        let out = run(src);
        assert!(
            !out.contains("\n      stranded"),
            "continuation held its stale column instead of rejoining: {out:?}",
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
    fn opening_continuation_joins_the_summary_below_it() {
        let src = "\"\"\"\n\\\nA summary left flush against the opener by a continuation that ran past the docstring budget.\n\"\"\"\n";
        let out = run(src);
        assert!(
            !out.contains('\\'),
            "a stranded continuation reached the output: {out:?}"
        );
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

    #[rstest]
    #[case("see https://host/\\\npath.html", false, &["see https://host/path.html"])]
    #[case("trailing run \\\n    indented", false, &["trailing run \\", "    indented"])]
    #[case("spaced out \\\nflush", false, &["spaced out \\", "flush"])]
    #[case("raw https://host/\\\npath.html", true, &["raw https://host/\\", "path.html"])]
    fn spliced_continuations_merges_only_a_join_carrying_no_whitespace(
        #[case] content: &str,
        #[case] raw: bool,
        #[case] expected: &[&str],
    ) {
        let lines: Vec<_> = spliced_continuations(content, "\n", raw)
            .into_iter()
            .map(|(_, line)| line)
            .collect();
        assert_eq!(lines, expected);
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

    #[test]
    fn typed_head_under_prose_reflows_into_the_paragraph() {
        let src = "def f():\n    \"\"\"\n    Short intro.\n    config (dict): more prose carrying the same paragraph.\n    \"\"\"\n    pass\n";
        assert!(
            run(src).contains("Short intro. config (dict):"),
            "a head with no blank line above it split the paragraph",
        );
    }

    #[rstest]
    #[case("plain prose", false, "plain prose")]
    #[case("escaped \\\\", false, "escaped \\\\")]
    #[case("continues \\", false, "continues ")]
    #[case("literal \\", true, "literal \\")]
    fn without_continuation_drops_only_an_odd_run_in_a_non_raw_body(
        #[case] line: &str,
        #[case] raw: bool,
        #[case] expected: &str,
    ) {
        assert_eq!(without_continuation(line, raw), expected);
    }
}
