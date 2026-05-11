//! Enforces the canonical multi-line docstring shape: opening `"""`
//! on its own line, body content, closing `"""` on its own line at
//! the docstring's indent. Detects two violations and rewrites both
//! with a single edit. Body content is preserved verbatim. Single-line
//! docstrings belong to the companion rule `no-single-line-docstrings`
//! and pass through this rule untouched.

use ruff_diagnostics::Edit;
use ruff_python_ast::StringLiteral;
use ruff_python_trivia::has_leading_content;

use crate::config::Config;
use crate::primitives::docstring::{indent_prefix, triple_quoted_body, DocstringHandler};
use crate::primitives::edit::narrowed_replacement;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct MultiLineDocstrings;

impl MultiLineDocstrings {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for MultiLineDocstrings {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut rewriter = Rewriter {
            edits: Vec::new(),
            source,
        };
        rewriter.walk(source);
        rewriter.edits
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(MultiLineDocstrings))
    }
}

struct Rewriter<'a> {
    edits: Vec<Edit>,
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
        let leading_ok = body.text.starts_with('\n') || body.text.starts_with('\r');
        let trailing_ok = !has_leading_content(body.range.end(), self.source.text());
        if leading_ok && trailing_ok {
            return;
        }
        let indent = indent_prefix(self.source, lit);
        let newline = self.source.newline_str();
        let mut new_body =
            String::with_capacity(body.text.len() + (newline.len() + indent.len()) * 2);
        if !leading_ok {
            new_body.push_str(newline);
            new_body.push_str(indent);
        }
        new_body.push_str(body.text);
        if !trailing_ok {
            new_body.push_str(newline);
            new_body.push_str(indent);
        }
        self.edits
            .extend(narrowed_replacement(self.source, body.range, new_body));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;
    use crate::test_support::parse;

    fn run(src: &str) -> String {
        let source = parse(src);
        let pipeline = Pipeline::for_rule("multi-line-docstrings", &Config::default())
            .expect("rule registered");
        pipeline
            .run(source)
            .expect("pipeline runs")
            .0
            .text()
            .to_owned()
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
