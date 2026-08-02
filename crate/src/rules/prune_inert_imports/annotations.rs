//! The names a quoted annotation reads. The binding table does not
//! reach them, their identifiers sitting inside a string literal rather
//! than in the AST.

use std::collections::HashSet;

use ruff_python_ast::{
    Expr, ModModule,
    visitor::{Visitor, walk_expr},
};
use ruff_python_parser::parse_expression;

use crate::primitives::walk::for_each_annotation;

/// Gathers every loaded name of a type expression into `names`, holding
/// each quoted member it carries for a further parse.
struct NameCollector<'a> {
    names: &'a mut HashSet<String>,
    nested: Vec<String>,
}

impl<'a> Visitor<'a> for NameCollector<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) if name.ctx.is_load() => {
                self.names.insert(name.id.to_string());
            }
            Expr::StringLiteral(literal) => self.nested.push(literal.value.to_str().to_owned()),
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// Every name a string-literal annotation in `module` reads, alongside
/// the unquoted names sharing those annotations, empty when the module
/// carries no annotation at all.
pub(super) fn annotation_names(module: &ModModule) -> HashSet<String> {
    let mut names = HashSet::new();
    for_each_annotation(&module.body, |annotation| absorb(annotation, &mut names));
    names
}

/// Records every name `annotation` loads, feeding each quoted member it
/// carries back through the parse. A literal that does not parse is
/// left alone.
fn absorb(annotation: &Expr, names: &mut HashSet<String>) {
    let mut pending = collect_names(annotation, names);
    while let Some(text) = pending.pop() {
        let Ok(parsed) = parse_expression(&text) else {
            continue;
        };
        pending.append(&mut collect_names(parsed.expr(), names));
    }
}

/// Adds every name `expr` loads to `names` and returns the text of each
/// string literal it carries.
fn collect_names(expr: &Expr, names: &mut HashSet<String>) -> Vec<String> {
    let mut collector = NameCollector {
        names,
        nested: Vec::new(),
    };
    collector.visit_expr(expr);
    collector.nested
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::whole_annotation("x: \"List[int]\" = []\n", &["List", "int"])]
    #[case::nested_member("x: dict[str, \"Node\"] = {}\n", &["Node", "dict", "str"])]
    #[case::doubly_quoted("x: \"List['Node']\" = []\n", &["List", "Node"])]
    #[case::literal_member("x: Literal[\"red\"] = \"red\"\n", &["Literal", "red"])]
    #[case::return_annotation("def f() -> \"Node\":\n    return None\n", &["Node"])]
    #[case::parameter_annotation("def f(a: \"Node\"):\n    pass\n", &["Node"])]
    #[case::unparseable_literal("x: \"not an expression!\" = 1\n", &[])]
    #[case::plain_string_value("x = \"List\"\n", &[])]
    #[case::unquoted_annotation("x: List[int] = []\n", &["List", "int"])]
    fn annotation_names_reads_each_quoted_form(#[case] src: &str, #[case] expected: &[&str]) {
        let source = parse(src);
        let mut names: Vec<String> = annotation_names(source.ast()).into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }
}
