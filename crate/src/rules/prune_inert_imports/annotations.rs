//! The names a quoted type expression reads, covering an annotation and
//! the typing calls that take a type as a string, which the binding
//! table does not reach.

use ruff_python_ast::{
    Expr, ExprCall, ModModule,
    visitor::{Visitor, walk_expr},
};
use ruff_python_parser::parse_expression;
use rustc_hash::FxHashSet;

use crate::primitives::walk::{Descent, filter_map_over_exprs, for_each_annotation};

/// The typing constructs that accept a type expression written as a
/// string. `cast` takes the type in its first argument and the value in
/// its second, whereas every other entry names the new type first and
/// takes a type expression in each argument after that name.
const TYPE_EXPRESSION_CALLS: [&str; 7] = [
    "NamedTuple",
    "NewType",
    "TypeAliasType",
    "TypeVar",
    "TypedDict",
    "assert_type",
    "cast",
];

/// Gathers every loaded name of a type expression into `names`, holding
/// each quoted member it carries for a further parse.
struct NameCollector<'a> {
    names: &'a mut FxHashSet<String>,
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

/// Every name a type expression in `module` loads, quoted or not,
/// covering an annotation and each typing call that takes its type as a
/// string. The set is empty where the module carries neither.
pub(super) fn type_expression_names(module: &ModModule) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for_each_annotation(&module.body, |annotation| absorb(annotation, &mut names));
    let quoted = filter_map_over_exprs(&module.body, Descent::Into, |expr| {
        Some(
            type_expression_args(expr.as_call_expr()?)?
                .flat_map(quoted_members)
                .collect::<Vec<_>>(),
        )
    });
    absorb_quoted(quoted.concat(), &mut names);
    names
}

/// Records every name `expr` loads, feeding each quoted member it
/// carries back through the parse.
fn absorb(expr: &Expr, names: &mut FxHashSet<String>) {
    absorb_quoted(collect_names(expr, names), names);
}

/// Parses each quoted type expression in `pending`, recording the names
/// it loads and following every literal nested inside it. A literal that
/// does not parse is left alone.
fn absorb_quoted(mut pending: Vec<String>, names: &mut FxHashSet<String>) {
    while let Some(text) = pending.pop() {
        let Ok(parsed) = parse_expression(&text) else {
            continue;
        };
        pending.append(&mut collect_names(parsed.expr(), names));
    }
}

/// Adds every name `expr` loads to `names` and returns the text of each
/// string literal it carries.
fn collect_names(expr: &Expr, names: &mut FxHashSet<String>) -> Vec<String> {
    let mut collector = NameCollector {
        names,
        nested: Vec::new(),
    };
    collector.visit_expr(expr);
    collector.nested
}

/// The text of every string literal `expr` carries at any depth.
fn quoted_members(expr: &Expr) -> Vec<String> {
    collect_names(expr, &mut FxHashSet::default())
}

/// Every argument of `call` that Python evaluates as a type expression,
/// returning `None` where the callee names no typing construct that
/// takes one.
fn type_expression_args(call: &ExprCall) -> Option<impl Iterator<Item = &Expr>> {
    let callee = match call.func.as_ref() {
        Expr::Attribute(attribute) => attribute.attr.as_str(),
        Expr::Name(name) => name.id.as_str(),
        _ => return None,
    };
    if !TYPE_EXPRESSION_CALLS.contains(&callee) {
        return None;
    }
    let (names, types) = if callee == "cast" {
        (0, 1)
    } else {
        (1, usize::MAX)
    };
    Some(
        call.arguments
            .args
            .iter()
            .skip(names)
            .take(types)
            .chain(call.arguments.keywords.iter().map(|keyword| &keyword.value)),
    )
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
    #[case::cast_argument("y = cast(\"IO[str]\", handle)\n", &["IO", "str"])]
    #[case::cast_value_argument("y = cast(\"IO[str]\", {\"k\": \"v\"})\n", &["IO", "str"])]
    #[case::qualified_cast_argument("y = typing.cast(\"IO[str]\", handle)\n", &["IO", "str"])]
    #[case::cast_inside_a_function("def f(h):\n    return cast(\"IO[str]\", h)\n", &["IO", "str"])]
    #[case::type_var_bound("T = TypeVar(\"T\", bound=\"IO[str]\")\n", &["IO", "str"])]
    #[case::type_var_constraints("T = TypeVar(\"T\", \"int\", \"str\")\n", &["int", "str"])]
    #[case::new_type_argument("H = NewType(\"H\", \"IO[str]\")\n", &["IO", "str"])]
    #[case::typed_dict_member(
        "D = TypedDict(\"D\", {\"handle\": \"IO[str]\"})\n",
        &["IO", "handle", "str"]
    )]
    #[case::named_tuple_member(
        "P = NamedTuple(\"P\", [(\"handle\", \"IO[str]\")])\n",
        &["IO", "handle", "str"]
    )]
    #[case::assert_type_argument("assert_type(handle, \"IO[str]\")\n", &["IO", "str"])]
    #[case::call_naming_no_typing_construct("y = open(\"IO[str]\")\n", &[])]
    #[case::call_through_a_subscript("y = builders[0](\"IO[str]\")\n", &[])]
    #[case::string_call_argument("y = print(\"List\")\n", &[])]
    fn type_expression_names_reads_each_quoted_form(#[case] src: &str, #[case] expected: &[&str]) {
        let source = parse(src);
        let mut names: Vec<String> = type_expression_names(source.ast()).into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }
}
