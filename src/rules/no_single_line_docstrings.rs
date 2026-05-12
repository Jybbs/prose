//! Rewrites a single-line triple-quoted docstring into the canonical
//! multi-line shape: opening `"""` on its own line, trimmed body on
//! its own line at the docstring's indent, closing `"""` on its own
//! line. Skips non-triple-quoted strings, inline `def f(): """..."""`
//! shapes, and bodies that trim to empty. The companion rule
//! `multi-line-docstrings` enforces the multi-line layout produced
//! here on every already-multi-line docstring.

use ruff_diagnostics::Edit;
use ruff_python_ast::StringLiteral;
use ruff_python_trivia::PythonWhitespace;

use crate::config::Config;
use crate::primitives::docstring::{indent_prefix, triple_quoted_body, DocstringHandler};
use crate::primitives::edit::narrowed_replacement;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct NoSingleLineDocstrings;

impl NoSingleLineDocstrings {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for NoSingleLineDocstrings {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut rewriter = Rewriter {
            edits: Vec::new(),
            source,
        };
        rewriter.walk(source);
        rewriter.edits
    }

    fn id(&self) -> RuleId {
        Self::SLUG
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
        if body.text.contains('\n') {
            return;
        }
        let trimmed = body.text.trim_whitespace();
        if trimmed.is_empty() {
            return;
        }
        let indent = indent_prefix(self.source, lit);
        let newline = self.source.newline_str();
        let candidate = format!("{newline}{indent}{trimmed}{newline}{indent}");
        self.edits
            .extend(narrowed_replacement(self.source, body.range, candidate));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;
    use crate::test_support::parse;

    fn run(src: &str) -> String {
        let source = parse(src);
        let pipeline = Pipeline::for_rule("no-single-line-docstrings", &Config::default())
            .expect("rule registered");
        pipeline
            .run(source)
            .expect("pipeline runs")
            .0
            .text()
            .to_owned()
    }

    #[test]
    fn disabled_via_config_leaves_single_line_docstring_intact() {
        let mut config = Config::default();
        config.rules.no_single_line_docstrings.enabled = false;
        let pipeline = Pipeline::with_defaults(&config);
        let source = parse("def f():\n    \"\"\"doc\"\"\"\n");
        let (out, _) = pipeline.run(source).expect("pipeline runs");
        assert_eq!(out.text(), "def f():\n    \"\"\"doc\"\"\"\n");
    }

    #[test]
    fn empty_body_after_trim_is_left_alone() {
        let src = "def f():\n    \"\"\"   \"\"\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn non_triple_quoted_single_line_is_left_alone() {
        let src = "def f():\n    \"summary\"\n";
        assert_eq!(run(src), src);
    }

    #[test]
    fn preserves_triple_single_quotes_in_rewrite() {
        assert_eq!(
            run("def f():\n    '''doc'''\n"),
            "def f():\n    '''\n    doc\n    '''\n",
        );
    }

    #[test]
    fn trims_leading_and_trailing_whitespace_from_body() {
        assert_eq!(
            run("def f():\n    \"\"\"  Summary.  \"\"\"\n"),
            "def f():\n    \"\"\"\n    Summary.\n    \"\"\"\n",
        );
    }
}
