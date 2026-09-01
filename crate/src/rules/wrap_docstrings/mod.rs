//! Wraps Google-style docstring prose to its configured budget:
//! description prose to `docstring_line_length`, Title-case-headed
//! sections to the budget `docstring_structured_policy` selects, and
//! each `name: description` entry to `docstring_line_length` with a
//! hanging indent, later lines opening no entry of their own gathered
//! into it. Every region [`LineScan`](crate::primitives::docstring::LineScan)
//! marks verbatim passes through
//! unchanged, reflowed prose collapses interior whitespace to one
//! space, and a backslash continuing a line of non-raw prose resolves
//! into the join rather than reaching the output as a word.

use ruff_diagnostics::Edit;
use ruff_text_size::{Ranged, TextSize};

use crate::{
    config::{Config, DocstringStructuredPolicy},
    primitives::{
        docstring::{DocstringBody, LineScanner, rewrite_docstrings, triple_quoted_body},
        edit::narrowed_replacement,
        padding,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod paragraph;
mod walk;
mod wrapping;

use paragraph::Paragraph;
use wrapping::spliced_continuations;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Region {
    Description,
    Section,
    SectionEntry,
}

#[derive(Debug)]
pub(crate) struct WrapDocstrings {
    pub(super) description_width: usize,
    pub(super) section_width: usize,
    stranding: padding::Stranding,
}

impl WrapDocstrings {
    pub(crate) const MESSAGE: &'static str = "wrap docstring prose to the configured budget";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

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

#[cfg(test)]
mod tests {
    use rstest::rstest;

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
}
