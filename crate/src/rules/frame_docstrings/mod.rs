//! Canonicalizes every docstring to the `"""` frame. A docstring in any
//! quote style (`'''`, `'...'`, `"..."`) is re-delimited to `"""` with
//! its prefix kept verbatim, unless a `"""` run in the body or a
//! single-line body ending in `"` would abut the closer. A multi-line
//! docstring additionally lands its opener and closer on their own lines
//! at the docstring's indent, the body between them preserved verbatim.
//! Single-line docstrings expand under the companion rule
//! `expand-docstrings`, which runs on the reframed `"""` result.

use ruff_diagnostics::Edit;
use ruff_python_ast::{StringFlags, StringLiteral};
use ruff_python_trivia::{PythonWhitespace, has_leading_content};
use ruff_text_size::TextRange;

use crate::{
    config::Config,
    primitives::{
        docstring::{DocstringBody, docstring_body, indent_prefix, rewrite_docstrings},
        edit::narrowed_replacement,
        quoting::{TRIPLE_QUOTE, abuts_triple_closer},
    },
    rule::{Preserves, Rule, RuleId},
    source::Source,
};

pub(crate) struct FrameDocstrings;

impl FrameDocstrings {
    pub(crate) const MESSAGE: &'static str =
        "canonicalize docstring quotes and frame the opener and closer on their own lines";

    pub(crate) const PRESERVES: Preserves = Preserves::Bindings;

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for FrameDocstrings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        rewrite_docstrings(source, |source, lit, edits| {
            let Some(body) = docstring_body(source, lit) else {
                return;
            };
            edits.extend(requote_edits(source, lit, &body));
            if !body.is_multiline() {
                return;
            }
            let leading_ok = body.text.starts_with(['\n', '\r']);
            let trailing_ok = !has_leading_content(body.range.end(), source.text());
            if leading_ok && trailing_ok {
                return;
            }
            let pad = format!("{}{}", source.newline_str(), indent_prefix(source, lit));
            let leading = if leading_ok { "" } else { pad.as_str() };
            let trailing = if trailing_ok { "" } else { pad.as_str() };
            let new_body = format!("{leading}{}{trailing}", body.text);
            edits.extend(narrowed_replacement(source, body.range, new_body));
        })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Edits re-delimiting `lit` to canonical `"""` with its prefix left in
/// place. Empty when `lit` already opens with `"""`, when the body is
/// blank, or when re-delimiting would break the string, in that a `"""`
/// run inside the body or a single-line body ending in `"` would abut
/// the closer.
fn requote_edits(source: &Source, lit: &StringLiteral, body: &DocstringBody) -> Vec<Edit> {
    let flags = lit.flags;
    if flags.quote_str() == TRIPLE_QUOTE
        || body.text.trim_whitespace().is_empty()
        || abuts_triple_closer(&[body.text], !body.is_multiline())
    {
        return Vec::new();
    }
    let opener = TextRange::new(body.range.start() - flags.quote_len(), body.range.start());
    let closer = TextRange::new(body.range.end(), body.range.end() + flags.closer_len());
    [opener, closer]
        .into_iter()
        .filter_map(|range| narrowed_replacement(source, range, TRIPLE_QUOTE.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::testing::run_rule;

    fn run(src: &str) -> String {
        run_rule("frame-docstrings", src)
    }

    #[test]
    fn blank_body_is_left_alone() {
        let src = "def f():\n    '  '\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn inline_single_quoted_docstring_is_left_alone() {
        let src = "def f(): 'doc'\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn multi_line_single_quoted_body_ending_in_quote_still_requotes() {
        assert_eq!(
            run("def f():\n    '''Line one.\n    Ends in quote\"'''\n"),
            "def f():\n    \"\"\"\n    Line one.\n    Ends in quote\"\n    \"\"\"\n",
        );
    }

    #[test]
    fn preserves_body_content_verbatim_including_inner_whitespace() {
        assert_eq!(
            run("def f():\n    \"\"\"  Summary.\n        indented\n    \"\"\"\n"),
            "def f():\n    \"\"\"\n      Summary.\n        indented\n    \"\"\"\n",
        );
    }

    #[test]
    fn raw_prefix_kept_on_requote() {
        assert_eq!(
            run("def f():\n    r\"summary\"\n"),
            "def f():\n    r\"\"\"summary\"\"\"\n",
        );
    }

    #[test]
    fn single_line_body_ending_in_quote_is_left_alone() {
        let src = "def f():\n    'ends\"'\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn single_line_double_quoted_requotes_to_triple() {
        assert_eq!(
            run("def f():\n    \"summary\"\n"),
            "def f():\n    \"\"\"summary\"\"\"\n",
        );
    }

    #[test]
    fn single_line_single_quoted_requotes_to_triple() {
        assert_eq!(
            run("def f():\n    'summary'\n"),
            "def f():\n    \"\"\"summary\"\"\"\n",
        );
    }

    #[test]
    fn single_line_triple_quoted_is_left_alone() {
        let src = "def f():\n    \"\"\"Summary.\"\"\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn triple_quote_run_in_body_is_left_alone() {
        let src = "def f():\n    'a \"\"\" b'\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn triple_single_quoted_multi_line_canonicalizes_to_double_quotes() {
        assert_eq!(
            run("def f():\n    '''Summary.\n    Trailing.\n    '''\n"),
            "def f():\n    \"\"\"\n    Summary.\n    Trailing.\n    \"\"\"\n",
        );
    }
}
