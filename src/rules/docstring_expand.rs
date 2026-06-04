//! Rewrites a single-line triple-quoted docstring into the canonical
//! multi-line shape: opening `"""` on its own line, trimmed body on
//! its own line at the docstring's indent, closing `"""` on its own
//! line. Skips non-triple-quoted strings, inline `def f(): """..."""`
//! shapes, and bodies that trim to empty. The companion rule
//! `docstring-frame` enforces the multi-line layout produced
//! here on every already-multi-line docstring.

use ruff_diagnostics::Edit;
use ruff_python_trivia::PythonWhitespace;

use crate::{
    config::Config,
    primitives::{
        docstring::{indent_prefix, rewrite_docstrings, triple_quoted_body},
        edit::{narrowed_replacement, singleton_groups},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct DocstringExpand;

impl DocstringExpand {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for DocstringExpand {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(rewrite_docstrings(source, |source, lit, edits| {
            let Some(body) = triple_quoted_body(source, lit).filter(|b| !b.is_multiline()) else {
                return;
            };
            let trimmed = body.text.trim_whitespace();
            if trimmed.is_empty() {
                return;
            }
            let indent = indent_prefix(source, lit);
            let newline = source.newline_str();
            let candidate = format!("{newline}{indent}{trimmed}{newline}{indent}");
            edits.extend(narrowed_replacement(source, body.range, candidate));
        }))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;
    use crate::testing::parse;

    fn run(src: &str) -> String {
        let source = parse(src);
        let pipeline =
            Pipeline::for_rule("docstring-expand", &Config::default()).expect("rule registered");
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
        config.rules.docstring_expand.enabled = false;
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
