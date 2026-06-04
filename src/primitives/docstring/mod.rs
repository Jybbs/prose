//! Shared walker for PEP 257 docstring statements, the first
//! body-statement of the module, each class, and each function that
//! holds a string literal as its first expression statement.
//! Implementors of [`DocstringHandler`] receive every such docstring
//! literal in source order via the trait's `walk` method. Implicitly
//! concatenated docstring expressions are skipped. The section
//! helpers `section_heading`, `entry_description_col`, and
//! `entry_carrying_sections` parse a docstring body's Title-case-headed
//! sections for consumers that walk text rather than the AST,
//! recognizing entry-carrying sections by content shape rather than
//! against a closed name list.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    ExprStringLiteral, Stmt, StringLiteral,
    statement_visitor::{StatementVisitor, walk_stmt},
};

use crate::source::Source;

mod body;
mod scan;
mod section;

pub(crate) use body::{indent_prefix, triple_quoted_body};
pub(crate) use scan::{LineScan, LineScanner};
pub(crate) use section::{entry_carrying_sections, entry_description_col, section_heading};

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

struct Visitor<'a, H: DocstringHandler> {
    handler: &'a mut H,
}

impl<H: DocstringHandler> Visitor<'_, H> {
    fn consider(&mut self, body: &[Stmt]) {
        let docstring = body
            .first()
            .and_then(Stmt::as_expr_stmt)
            .and_then(|e| e.value.as_string_literal_expr())
            .and_then(ExprStringLiteral::as_single_part_string);
        if let Some(lit) = docstring {
            self.handler.handle(lit);
        }
    }
}

impl<'a, H: DocstringHandler> StatementVisitor<'a> for Visitor<'_, H> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(c) => self.consider(&c.body),
            Stmt::FunctionDef(f) => self.consider(&f.body),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Walks every docstring in `source` and collects the edits produced
/// by `f` against each. The closure receives `source`, the docstring
/// literal, and the running edit buffer. Returns the accumulated edits.
pub(crate) fn rewrite_docstrings<F>(source: &Source, f: F) -> Vec<Edit>
where
    F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
{
    struct Collector<'a, F> {
        edits: Vec<Edit>,
        f: F,
        source: &'a Source,
    }

    impl<F> DocstringHandler for Collector<'_, F>
    where
        F: FnMut(&Source, &StringLiteral, &mut Vec<Edit>),
    {
        fn handle(&mut self, lit: &StringLiteral) {
            (self.f)(self.source, lit, &mut self.edits);
        }
    }

    let mut collector = Collector {
        edits: Vec::new(),
        f,
        source,
    };
    collector.walk(source);
    collector.edits
}

#[cfg(test)]
mod tests {
    use ruff_text_size::Ranged;

    use super::*;
    use crate::testing::parse;

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
    fn rewrite_docstrings_collects_edits_pushed_by_closure_per_docstring() {
        let s = parse("\"\"\"M\"\"\"\ndef f():\n    \"\"\"f\"\"\"\n    pass\n");
        let edits = rewrite_docstrings(&s, |_, lit, edits| {
            edits.push(Edit::range_deletion(lit.range()));
        });
        assert_eq!(edits.len(), 2);
        assert!(edits.windows(2).all(|w| w[0].start() < w[1].start()));
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
