//! Shared walker for PEP 257 docstring statements, the first
//! body-statement of the module, each class, and each function that
//! satisfies `is_docstring_stmt`. Implementors of [`DocstringHandler`]
//! receive every such docstring literal in source order via the
//! trait's `walk` method. Implicitly concatenated docstring
//! expressions are skipped, since their rewrite shape is not defined.

use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::{Stmt, StringFlags, StringLiteral};
use ruff_python_trivia::{has_leading_content, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::source::Source;

/// Body slice between a triple-quoted docstring's opener and closer,
/// paired with the source range that slice covers.
pub(crate) struct DocstringBody<'a> {
    pub(crate) range: TextRange,
    pub(crate) text: &'a str,
}

/// Receiver for the docstring walker. Implementors handle each
/// docstring `StringLiteral` reached in source order. Call `walk`
/// to drive the receiver across `source`'s module body.
pub(crate) trait DocstringHandler {
    fn handle(&mut self, lit: &StringLiteral);

    fn walk(&mut self, source: &Source)
    where
        Self: Sized,
    {
        let mut visitor = Visitor { handler: self };
        let body = &source.ast().body;
        visitor.consider(body);
        visitor.visit_body(body);
    }
}

struct Visitor<'a, H: DocstringHandler + ?Sized> {
    handler: &'a mut H,
}

impl<H: DocstringHandler + ?Sized> Visitor<'_, H> {
    fn consider(&mut self, body: &[Stmt]) {
        let docstring = body
            .first()
            .filter(|s| is_docstring_stmt(s))
            .and_then(|s| s.as_expr_stmt())
            .and_then(|e| e.value.as_string_literal_expr())
            .filter(|v| !v.value.is_implicit_concatenated())
            .and_then(|v| v.value.iter().next());
        if let Some(lit) = docstring {
            self.handler.handle(lit);
        }
    }
}

impl<'a, H: DocstringHandler + ?Sized> StatementVisitor<'a> for Visitor<'_, H> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(c) => self.consider(&c.body),
            Stmt::FunctionDef(f) => self.consider(&f.body),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the line indent prefix of the docstring at `lit.start()`,
/// preserving the source's mix of tabs and spaces verbatim.
pub(crate) fn indent_prefix<'a>(source: &'a Source, lit: &StringLiteral) -> &'a str {
    leading_indentation(source.text().line_str(lit.start()))
}

/// Returns the body slice and source range when `lit` is triple-quoted
/// and sits at the start of its own line. Returns `None` for
/// non-triple-quoted literals and for inline `def f(): """..."""`
/// shapes, where the rewrite shape is not defined.
pub(crate) fn triple_quoted_body<'a>(
    source: &'a Source,
    lit: &StringLiteral,
) -> Option<DocstringBody<'a>> {
    if !lit.flags.is_triple_quoted() || has_leading_content(lit.start(), source.text()) {
        return None;
    }
    let range = TextRange::new(
        lit.start() + lit.flags.opener_len(),
        lit.end() - lit.flags.closer_len(),
    );
    Some(DocstringBody {
        range,
        text: source.slice(range),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    #[derive(Default)]
    struct Probe<'a> {
        source: Option<&'a Source>,
        values: Vec<String>,
        bodies: Vec<String>,
        indents: Vec<String>,
    }

    impl Probe<'_> {
        fn run(source: &Source) -> Vec<String> {
            let mut probe = Probe::default();
            probe.walk(source);
            probe.values
        }
    }

    impl DocstringHandler for Probe<'_> {
        fn handle(&mut self, lit: &StringLiteral) {
            self.values.push(lit.value.to_string());
            if let Some(source) = self.source {
                self.indents.push(indent_prefix(source, lit).to_owned());
                self.bodies
                    .extend(triple_quoted_body(source, lit).map(|b| b.text.to_owned()));
            }
        }
    }

    fn probe_with_source(source: &Source) -> Probe<'_> {
        let mut probe = Probe {
            source: Some(source),
            ..Probe::default()
        };
        probe.walk(source);
        probe
    }

    #[test]
    fn collects_class_function_and_method_docstrings_in_source_order() {
        let s = parse(
            "\"\"\"M\"\"\"\nclass C:\n    \"\"\"C\"\"\"\n    def m(self):\n        \"\"\"m\"\"\"\n        pass\n",
        );
        assert_eq!(Probe::run(&s), ["M", "C", "m"]);
    }

    #[test]
    fn collects_nested_function_docstrings() {
        let s = parse(
            "def outer():\n    \"\"\"o\"\"\"\n    def inner():\n        \"\"\"i\"\"\"\n        pass\n",
        );
        assert_eq!(Probe::run(&s), ["o", "i"]);
    }

    #[test]
    fn indent_prefix_preserves_source_indent_characters() {
        let s = parse("class C:\n\t\"\"\"doc\"\"\"\n\tpass\n");
        let probe = probe_with_source(&s);
        assert_eq!(probe.indents, ["\t"]);
    }

    #[test]
    fn returns_empty_for_module_with_no_docstrings() {
        let s = parse("x = 1\ndef f():\n    return 1\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn skips_implicitly_concatenated_docstring_expressions() {
        let s = parse("\"\"\"a\"\"\" \"\"\"b\"\"\"\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn skips_string_expression_that_is_not_first_statement() {
        let s = parse("x = 1\n\"not a docstring\"\n");
        assert!(Probe::run(&s).is_empty());
    }

    #[test]
    fn triple_quoted_body_extracts_inner_body_text() {
        let s = parse("'''hello'''\n");
        let probe = probe_with_source(&s);
        assert_eq!(probe.bodies, ["hello"]);
    }

    #[test]
    fn triple_quoted_body_rejects_inline_with_def() {
        let s = parse("def f(): \"\"\"doc\"\"\"\n");
        let probe = probe_with_source(&s);
        assert!(probe.bodies.is_empty());
    }

    #[test]
    fn triple_quoted_body_rejects_non_triple_quoted_literal() {
        let s = parse("\"hello\"\n");
        let probe = probe_with_source(&s);
        assert!(probe.bodies.is_empty());
    }
}
