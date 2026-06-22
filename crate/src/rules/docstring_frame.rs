//! Enforces the canonical multi-line docstring shape: opening `"""`
//! on its own line, body content, closing `"""` on its own line at
//! the docstring's indent. Detects two violations and rewrites both
//! with a single edit. Body content is preserved verbatim. Single-line
//! docstrings belong to the companion rule `docstring-expand`
//! and pass through this rule untouched.

use ruff_diagnostics::Edit;
use ruff_python_trivia::has_leading_content;

use crate::{
    config::Config,
    primitives::{
        docstring::{DocstringBody, indent_prefix, rewrite_docstrings, triple_quoted_body},
        edit::{narrowed_replacement, singleton_groups},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct DocstringFrame;

impl DocstringFrame {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for DocstringFrame {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(rewrite_docstrings(source, |source, lit, edits| {
            let Some(body) = triple_quoted_body(source, lit).filter(DocstringBody::is_multiline)
            else {
                return;
            };
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
        }))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::run_rule;

    fn run(src: &str) -> String {
        run_rule("docstring-frame", src)
    }

    #[test]
    fn preserves_body_content_verbatim_including_inner_whitespace() {
        assert_eq!(
            run("def f():\n    \"\"\"  Summary.\n        indented\n    \"\"\"\n"),
            "def f():\n    \"\"\"\n      Summary.\n        indented\n    \"\"\"\n",
        );
    }

    #[test]
    fn single_line_docstring_is_left_alone() {
        let src = "def f():\n    \"\"\"Summary.\"\"\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn triple_single_quoted_multi_line_normalizes_under_its_own_quote_style() {
        assert_eq!(
            run("def f():\n    '''Summary.\n    Trailing.\n    '''\n"),
            "def f():\n    '''\n    Summary.\n    Trailing.\n    '''\n",
        );
    }
}
