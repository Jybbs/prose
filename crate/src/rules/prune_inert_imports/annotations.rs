//! The names a quoted type expression reads, covering an annotation, an
//! explicit type alias, the typing calls that take a type as a string,
//! and the subscript of a standard-library generic, none of which the
//! binding table reaches.

use ruff_python_ast::{
    Expr, ExprCall, ExprSubscript, ModModule, Stmt,
    helpers::map_subscript,
    visitor::{Visitor, walk_expr},
};
use ruff_python_parser::parse_expression;
use ruff_python_stdlib::typing::{is_pep_593_generic_member, is_standard_library_generic_member};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::primitives::{
    binding::{from_import_bound_name, is_explicit_type_alias, tail_identifier},
    walk::{Descent, filter_map_over_exprs, filter_map_over_stmts, for_each_annotation},
};

/// The name each `from`-import alias binds, against the member it takes
/// out of its module, which resolves a construct renamed on the way in
/// back to the construct it names.
type Aliases<'a> = FxHashMap<&'a str, &'a str>;

/// The typing constructs that accept a type expression written as a
/// string. `cast` takes the type in its first argument and the value in
/// its second, whereas every other entry opens with a name or a value
/// and takes a type expression in each argument after it.
const TYPE_EXPRESSION_CALLS: [&str; 9] = [
    "NamedTuple",
    "NewType",
    "ParamSpec",
    "TypeAliasType",
    "TypeVar",
    "TypeVarTuple",
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
/// covering an annotation, an explicit type alias, each typing call that
/// takes its type as a string, and the subscript of a standard-library
/// generic. The set is empty where the module carries none of them.
pub(super) fn type_expression_names(module: &ModModule) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for_each_annotation(&module.body, |annotation| absorb(annotation, &mut names));
    for value in filter_map_over_stmts(&module.body, alias_value) {
        absorb(value, &mut names);
    }
    let aliases = import_aliases(module);
    let quoted = filter_map_over_exprs(&module.body, Descent::Into, |expr| match expr {
        Expr::Call(call) => Some(
            type_expression_args(call, &aliases)?
                .flat_map(quoted_members)
                .collect::<Vec<_>>(),
        ),
        Expr::Subscript(subscript) => Some(quoted_members(generic_slice(subscript, &aliases)?)),
        _ => None,
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

/// The value an explicit type alias binds, which Python reads as a type
/// expression whether it is written bare or as a string. `None` for
/// every other statement and for a `TypeAlias` annotation with no value.
fn alias_value(stmt: &Stmt) -> Option<&Expr> {
    if !is_explicit_type_alias(stmt) {
        return None;
    }
    match stmt {
        Stmt::AnnAssign(node) => node.value.as_deref(),
        Stmt::TypeAlias(node) => Some(node.value.as_ref()),
        _ => None,
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

/// The typing construct `expr` names at its head, read past a subscript
/// and back through the `from`-import that bound it, so `cast`,
/// `typing.cast`, and a `cast as c` alias all read as `cast`.
fn construct<'a>(expr: &'a Expr, aliases: &Aliases<'a>) -> Option<&'a str> {
    let head = map_subscript(expr);
    let written = tail_identifier(head)?;
    Some(match head {
        Expr::Name(_) => aliases.get(written).copied().unwrap_or(written),
        _ => written,
    })
}

/// The slice of `subscript` where its head names a standard-library
/// generic, which takes each member of that slice as a type expression.
fn generic_slice<'a>(subscript: &'a ExprSubscript, aliases: &Aliases<'a>) -> Option<&'a Expr> {
    let head = construct(&subscript.value, aliases)?;
    (is_standard_library_generic_member(head) || is_pep_593_generic_member(head))
        .then_some(subscript.slice.as_ref())
}

/// The name every `from`-import in `module` binds, against the member it
/// names at its source.
fn import_aliases(module: &ModModule) -> Aliases<'_> {
    filter_map_over_stmts(&module.body, Stmt::as_import_from_stmt)
        .into_iter()
        .flat_map(|node| node.names.iter())
        .map(|alias| (from_import_bound_name(alias), alias.name.as_str()))
        .collect()
}

/// The text of every string literal `expr` carries at any depth.
fn quoted_members(expr: &Expr) -> Vec<String> {
    collect_names(expr, &mut FxHashSet::default())
}

/// Every argument of `call` that Python evaluates as a type expression,
/// returning `None` where the callee names no typing construct that
/// takes one.
fn type_expression_args<'a>(
    call: &'a ExprCall,
    aliases: &Aliases<'a>,
) -> Option<impl Iterator<Item = &'a Expr>> {
    let callee = construct(&call.func, aliases)?;
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
    #[case::aliased_cast_callee(
        "from typing import cast as c\n\ny = c(\"IO[str]\", handle)\n",
        &["IO", "str"]
    )]
    #[case::cast_inside_a_function("def f(h):\n    return cast(\"IO[str]\", h)\n", &["IO", "str"])]
    #[case::type_var_bound("T = TypeVar(\"T\", bound=\"IO[str]\")\n", &["IO", "str"])]
    #[case::type_var_constraints("T = TypeVar(\"T\", \"int\", \"str\")\n", &["int", "str"])]
    #[case::param_spec_bound("P = ParamSpec(\"P\", bound=\"IO[str]\")\n", &["IO", "str"])]
    #[case::type_var_tuple_default(
        "Ts = TypeVarTuple(\"Ts\", default=\"IO[str]\")\n",
        &["IO", "str"]
    )]
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
    #[case::string_call_argument("y = print(\"List\")\n", &[])]
    #[case::call_through_a_subscript("y = builders[0](\"IO[str]\")\n", &[])]
    #[case::generic_alias_value("Handle = Optional[\"IO[str]\"]\n", &["IO", "str"])]
    #[case::builtin_generic_alias("Vec = list[\"IO[str]\"]\n", &["IO", "str"])]
    #[case::qualified_generic_alias("Handle = typing.Optional[\"IO[str]\"]\n", &["IO", "str"])]
    #[case::aliased_generic_head(
        "from typing import Optional as Opt\n\nHandle = Opt[\"IO[str]\"]\n",
        &["IO", "str"]
    )]
    #[case::annotated_alias_value("Handle = Annotated[\"IO[str]\", meta]\n", &["IO", "str"])]
    #[case::annotated_string_metadata(
        "Handle = Annotated[\"IO[str]\", \"note\"]\n",
        &["IO", "note", "str"]
    )]
    #[case::annotated_type_alias("Handle: TypeAlias = \"IO[str]\"\n", &["IO", "TypeAlias", "str"])]
    #[case::pep_695_type_alias("type Handle = \"IO[str]\"\n", &["IO", "str"])]
    #[case::annotated_assignment_of_a_string("x: str = \"IO[str]\"\n", &["str"])]
    #[case::dict_lookup_names_no_type("y = config[\"Node\"]\n", &[])]
    #[case::attribute_dict_lookup("y = os.environ[\"Node\"]\n", &[])]
    #[case::literal_alias_value("Color = Literal[\"red\"]\n", &[])]
    fn type_expression_names_reads_each_quoted_form(#[case] src: &str, #[case] expected: &[&str]) {
        let source = parse(src);
        let mut names: Vec<String> = type_expression_names(source.ast()).into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }
}
