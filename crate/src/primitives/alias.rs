//! Tells a bare PEP 484 type alias apart from a module constant.
//!
//! Three checks run, and none can turn a constant into an alias, only
//! an alias into a constant, so a value none of them pins down stays an
//! alias and draws no rename.
//!
//! `runtime_only` matches the slice of a subscript against what a type
//! expression may hold, so `path_separators[1:]` is an index. Nothing
//! in a type is a slice, a dunder, a float, a call, or a comprehension,
//! and a bare integer, bool, or bytes stands only inside `Literal`.
//!
//! `name_evidence` looks the base of a subscript up in the module
//! binding table, so `SETTINGS["db"]` is a lookup once `SETTINGS` is
//! found assigned a dict display in the same file, and `Box[int]` is a
//! type once `Box` is found declared by a `class` statement. An
//! imported base yields `Unknown` and stays a type.
//!
//! `is_type_alias` adds the read contexts of the bound name, so a name
//! the module truth-tests, order-compares, or does arithmetic on holds
//! data, whereas one read in an annotation names a type.
//! `band-constants` calls `value_is_alias` and skips this last check,
//! sorting its sub-band on the value alone.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{
    Expr, ExprAttribute, ExprBinOp, ExprNumberLiteral, ExprSubscript, ExprUnaryOp, Number,
    Operator, Stmt, UnaryOp, helpers::is_dunder, name::UnqualifiedName,
};
use ruff_python_stdlib::typing::{is_literal_member, is_pep_593_generic_member};

use super::binding::{
    BindingAnalysis, BindingKind, ModuleAssignment, ReadContext, module_assignments,
};

/// What the module proves about the object a name holds.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Evidence {
    /// The module builds the value out of data it constructs.
    Runtime,
    /// A `class` statement in the module binds the name.
    Type,
    /// The module settles nothing, so the name may hold either.
    Unknown,
}

/// The module-scope evidence the alias read consumes, pairing the
/// binding table with every module-scope assignment and an index from
/// each bound name to its value.
pub(crate) struct AliasContext<'src> {
    analysis: &'src BindingAnalysis,
    sites: Vec<ModuleAssignment<'src>>,
    values: HashMap<&'src str, &'src Expr>,
}

impl<'src> AliasContext<'src> {
    pub(crate) fn new(body: &'src [Stmt], analysis: &'src BindingAnalysis) -> Self {
        let sites = module_assignments(body);
        let values = sites
            .iter()
            .filter_map(|site| Some((site.target.id.as_str(), site.value?)))
            .collect();
        Self {
            analysis,
            sites,
            values,
        }
    }

    /// Every module-scope single-name assignment, in source order.
    pub(crate) fn sites(&self) -> &[ModuleAssignment<'src>] {
        &self.sites
    }
}

/// True when `site` binds a type. The value must name an object that
/// already exists and no read of the bound name may prove it holds data.
pub(crate) fn is_type_alias<'src>(site: &ModuleAssignment<'src>, ctx: &AliasContext<'src>) -> bool {
    site.value.is_some_and(|value| value_is_alias(value, ctx))
        && ctx.analysis.module_read_context(site.target.id.as_str()) != ReadContext::Runtime
}

/// True when `value` names an object that already exists rather than
/// constructing a new one. A `|` union holds only when both sides do,
/// leaving `int | float` an alias and `1 | 2` a constant.
pub(crate) fn value_is_alias<'src>(value: &'src Expr, ctx: &AliasContext<'src>) -> bool {
    alias_value(value, ctx, &mut HashSet::new())
}

fn alias_value<'src>(
    value: &'src Expr,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> bool {
    match value {
        Expr::BinOp(ExprBinOp {
            left,
            op: Operator::BitOr,
            right,
            ..
        }) => union_arm(left, ctx, visited) && union_arm(right, ctx, visited),
        Expr::Subscript(subscript) => !subscript_is_runtime(subscript, ctx, visited),
        _ => UnqualifiedName::from_expr(value)
            .is_some_and(|base| dotted_is_alias(&base, ctx, visited)),
    }
}

/// True when a dotted name may hold a type. A dunder anywhere along the
/// path names a runtime value (`__builtins__`, `sys.__stdout__`), and a
/// head the module binds to data holds data.
fn dotted_is_alias<'src>(
    base: &UnqualifiedName<'_>,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> bool {
    !base.segments().iter().any(|segment| is_dunder(segment))
        && head_evidence(base, ctx, visited) != Evidence::Runtime
}

/// True when `arm` is a type a PEP 604 union may carry, an alias value
/// or the `None` of an optional. A bare `None` outside a union binds a
/// sentinel rather than naming a type.
fn union_arm<'src>(
    arm: &'src Expr,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> bool {
    arm.is_none_literal_expr() || alias_value(arm, ctx, visited)
}

/// True when `subscript` indexes a value at runtime rather than
/// parametrizing a type, proven either by the module binding the base to
/// data or by a node the typing grammar never admits in the slice.
fn subscript_is_runtime<'src>(
    subscript: &'src ExprSubscript,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> bool {
    let Some(base) = UnqualifiedName::from_expr(&subscript.value) else {
        // A call or a chained subscript roots the base, as in
        // `get_registry().default` and `registry[0][1]`.
        return true;
    };
    if !dotted_is_alias(&base, ctx, visited) {
        return true;
    }
    let slice = subscript.slice.as_ref();
    match base.segments().last().copied() {
        // PEP 586 admits an int, a bool, and bytes, and `Literal` is the
        // sole construct that carries them.
        Some(tail) if is_literal_member(tail) => runtime_only(slice, ctx, visited, true),
        // PEP 593 leaves every argument after the first arbitrary, so
        // `Annotated[int, Field(gt=0)]` reads only its type.
        Some(tail) if is_pep_593_generic_member(tail) => match slice {
            Expr::Tuple(tuple) => tuple
                .elts
                .first()
                .is_some_and(|annotated| runtime_only(annotated, ctx, visited, false)),
            single => runtime_only(single, ctx, visited, false),
        },
        _ => runtime_only(slice, ctx, visited, false),
    }
}

/// True when `expr` cannot stand at this position in a type expression,
/// which proves the enclosing subscript indexes data. `in_literal`
/// widens the grammar to the arguments a `Literal[...]` carries.
///
/// The match is exhaustive over `Expr` rather than closing on a
/// wildcard, so a new AST node raises a compile error instead of
/// silently reading as a type.
fn runtime_only<'src>(
    expr: &'src Expr,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
    in_literal: bool,
) -> bool {
    match expr {
        // No type expression admits any of these.
        Expr::Await(_)
        | Expr::BoolOp(_)
        | Expr::Call(_)
        | Expr::Compare(_)
        | Expr::Dict(_)
        | Expr::DictComp(_)
        | Expr::FString(_)
        | Expr::Generator(_)
        | Expr::If(_)
        | Expr::IpyEscapeCommand(_)
        | Expr::Lambda(_)
        | Expr::ListComp(_)
        | Expr::Named(_)
        | Expr::Set(_)
        | Expr::SetComp(_)
        | Expr::Slice(_)
        | Expr::TString(_)
        | Expr::Yield(_)
        | Expr::YieldFrom(_) => true,

        // Only the PEP 604 union, and only when both arms hold types.
        Expr::BinOp(ExprBinOp {
            left, op, right, ..
        }) => {
            *op != Operator::BitOr
                || runtime_only(left, ctx, visited, in_literal)
                || runtime_only(right, ctx, visited, in_literal)
        }

        // Only the signed integer of `Literal[-1]`.
        Expr::UnaryOp(ExprUnaryOp { op, operand, .. }) => !matches!(
            (op, operand.as_ref()),
            (
                UnaryOp::UAdd | UnaryOp::USub,
                Expr::NumberLiteral(ExprNumberLiteral {
                    value: Number::Int(_),
                    ..
                })
            )
        ),

        // An int, a bool, and bytes ride on `Literal` alone.
        Expr::BooleanLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(ExprNumberLiteral {
            value: Number::Int(_),
            ..
        }) => !in_literal,
        // A float and a complex have no place in any type expression.
        Expr::NumberLiteral(_) => true,

        // A dunder names a runtime value, and a name the module binds to
        // data indexes it (`table[idx]` under `idx = 0`).
        Expr::Name(name) => {
            is_dunder(&name.id)
                || name_evidence(name.id.as_str(), ctx, visited) == Evidence::Runtime
        }
        Expr::Attribute(ExprAttribute { attr, value, .. }) => {
            is_dunder(attr) || runtime_only(value, ctx, visited, in_literal)
        }

        // A container inherits the proof its members carry.
        Expr::List(list) => list
            .elts
            .iter()
            .any(|element| runtime_only(element, ctx, visited, in_literal)),
        Expr::Tuple(tuple) => tuple
            .elts
            .iter()
            .any(|element| runtime_only(element, ctx, visited, in_literal)),
        Expr::Starred(starred) => runtime_only(&starred.value, ctx, visited, in_literal),
        Expr::Subscript(subscript) => subscript_is_runtime(subscript, ctx, visited),

        // A string is a forward reference, `None` is a type, and `...`
        // is the shape of `Callable[..., int]`.
        Expr::EllipsisLiteral(_) | Expr::NoneLiteral(_) | Expr::StringLiteral(_) => false,
    }
}

/// The evidence the module carries about the head of a dotted base, the
/// `sys` of `sys.modules` and the `SETTINGS` of `SETTINGS["db"]`.
fn head_evidence<'src>(
    base: &UnqualifiedName<'_>,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> Evidence {
    base.segments()
        .first()
        .map_or(Evidence::Unknown, |head| name_evidence(head, ctx, visited))
}

/// The evidence the module carries about what `name` holds. A name the
/// module never binds, imports, or rebinds settles nothing, because the
/// object it holds is decided outside this file.
fn name_evidence<'src>(
    name: &str,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> Evidence {
    let kinds = ctx.analysis.module_binding_kinds(name);
    if kinds.is_empty() {
        // A builtin, a star-import, or a runtime-injected global.
        return Evidence::Unknown;
    }
    if kinds.contains(&BindingKind::ClassDef) {
        return Evidence::Type;
    }
    if kinds.contains(&BindingKind::Import) || ctx.analysis.module_reassigned(name) {
        return Evidence::Unknown;
    }
    let Some((&key, &value)) = ctx.values.get_key_value(name) else {
        return Evidence::Unknown;
    };
    if !visited.insert(key) {
        // A cycle, as in `A = A[0]`.
        return Evidence::Unknown;
    }
    value_evidence(value, ctx, visited)
}

/// The evidence a bound value carries. A call binds a type as readily as
/// data (`T = TypeVar("T")`, `Point = namedtuple(...)`), and an
/// attribute is undecidable in the module alone (`opener = TarFile.open`
/// against `path_sep = os.sep`), so both settle nothing.
fn value_evidence<'src>(
    value: &'src Expr,
    ctx: &AliasContext<'src>,
    visited: &mut HashSet<&'src str>,
) -> Evidence {
    match value {
        Expr::Attribute(_) | Expr::Call(_) => Evidence::Unknown,
        _ if alias_value(value, ctx, visited) => Evidence::Type,
        _ => Evidence::Runtime,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    /// Classifies the value of the last module-scope assignment in `src`,
    /// so a case may bind whatever names the value resolves against.
    fn last_value_is_alias(src: &str) -> bool {
        let source = parse(src);
        let analysis = source.binding_analysis();
        let ctx = AliasContext::new(&source.ast().body, analysis);
        let site = ctx.sites().last().expect("a module assignment");
        value_is_alias(site.value.expect("a value"), &ctx)
    }

    #[rstest]
    #[case::bare_name("X = Turtle", true)]
    #[case::attribute("X = TarFile.open", true)]
    #[case::builtin_generic("X = list[float]", true)]
    #[case::typing_union("X = Union[int, float]", true)]
    #[case::qualified("X = typing.Optional[int]", true)]
    #[case::dotted_base("X = collections.abc.Sequence[int]", true)]
    #[case::literal_strings("X = Literal[\"read\", \"write\"]", true)]
    #[case::literal_ints("X = Literal[1, 2]", true)]
    #[case::literal_signed("X = Literal[-1]", true)]
    #[case::literal_bool("X = Literal[True]", true)]
    #[case::literal_bytes("X = Literal[b\"raw\"]", true)]
    #[case::pep604("X = int | float", true)]
    #[case::pep604_optional("X = int | None", true)]
    #[case::pep604_chain("X = str | None | bytes", true)]
    #[case::pep604_generics("X = dict[str, int] | list[str]", true)]
    #[case::forward_ref("X = list[\"Node\"]", true)]
    #[case::callable("X = Callable[[int, str], bool]", true)]
    #[case::callable_ellipsis("X = Callable[..., int]", true)]
    #[case::empty_tuple("X = tuple[()]", true)]
    #[case::pep646_starred("X = tuple[*Ts]", true)]
    #[case::pep593_annotated("X = Annotated[int, Field(gt=0)]", true)]
    #[case::third_party_generic("X = NDArray[float]", true)]
    #[case::nested_union_generic("X = dict[str, int | None]", true)]
    #[case::number("X = 42", false)]
    #[case::none("X = None", false)]
    #[case::string("X = \"MyClass\"", false)]
    #[case::fstring("X = f\"{prefix}-suffix\"", false)]
    #[case::list("X = [1, 2]", false)]
    #[case::dict("X = {\"a\": 1}", false)]
    #[case::tuple("X = (int, str)", false)]
    #[case::int_union("X = 1 | 2", false)]
    #[case::mixed_union("X = int | 2", false)]
    #[case::arithmetic("X = BASE * 2", false)]
    #[case::call("X = make()", false)]
    #[case::lambda("X = lambda row: row.id", false)]
    #[case::call_rooted_attribute("X = get_registry().default", false)]
    #[case::call_rooted_subscript("X = load()[0]", false)]
    #[case::chained_subscript("X = registry[0][1]", false)]
    fn value_shape_splits_named_objects_from_constructed_data(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(last_value_is_alias(src), expected, "{src}");
    }

    #[rstest]
    #[case::slice_index("X = path_separators[1:]", false)]
    #[case::bare_slice("X = data[:]", false)]
    #[case::stepped_slice("X = data[::2]", false)]
    #[case::dunder_slice("X = sys.modules[__name__]", false)]
    #[case::dunder_base("X = __builtins__[\"open\"]", false)]
    #[case::int_index("X = path_separators[0]", false)]
    #[case::bool_index("X = options[True]", false)]
    #[case::float_index("X = table[3.14]", false)]
    #[case::call_slice("X = config[name.upper()]", false)]
    #[case::arithmetic_slice("X = row[index + 1]", false)]
    #[case::fstring_slice("X = table[f\"{key}-row\"]", false)]
    #[case::comparison_slice("X = frame[column > 3]", false)]
    #[case::comprehension_slice("X = frame[[c for c in cols]]", false)]
    fn the_typing_grammar_rejects_a_runtime_index(#[case] src: &str, #[case] expected: bool) {
        assert_eq!(last_value_is_alias(src), expected, "{src}");
    }

    #[rstest]
    #[case::dict_display("SETTINGS = {\"db\": \"pg\"}\nX = SETTINGS[\"db\"]", false)]
    #[case::list_display("ROWS = [1, 2]\nX = ROWS[0]", false)]
    #[case::string_value("BASE = \"abc\"\nX = BASE[0]", false)]
    #[case::indirect_dict("A = {\"k\": 1}\nB = A\nX = B[\"k\"]", false)]
    #[case::bare_reference("SETTINGS = {\"db\": \"pg\"}\nX = SETTINGS", false)]
    #[case::runtime_name_slice("IDX = 0\nX = table[IDX]", false)]
    #[case::class_generic("class Box(Generic[T]):\n    pass\nX = Box[int]", true)]
    #[case::imported_generic("from numpy.typing import NDArray\nX = NDArray[float]", true)]
    #[case::imported_literal("from typing import Literal\nX = Literal[\"read\"]", true)]
    #[case::local_alias_slice("Key = int\nX = dict[Key, str]", true)]
    #[case::reassigned_base("A = {}\nA = load()\nX = A[\"k\"]", true)]
    #[case::self_indexing_base("A = A[0]\nX = A[1]", false)]
    #[case::mutually_cyclic_bases("A = B[int]\nB = A[int]\nX = A[str]", true)]
    fn the_module_binding_settles_the_base(#[case] src: &str, #[case] expected: bool) {
        assert_eq!(last_value_is_alias(src), expected, "{src}");
    }

    /// Classifies the first module-scope assignment, reading how the
    /// rest of the module uses the name it binds.
    fn first_is_type_alias(src: &str) -> bool {
        let source = parse(src);
        let analysis = source.binding_analysis();
        let ctx = AliasContext::new(&source.ast().body, analysis);
        is_type_alias(ctx.sites().first().expect("a module assignment"), &ctx)
    }

    #[rstest]
    #[case::unread("Mode = Literal[\"read\"]", true)]
    #[case::annotated_parameter("Mode = Literal[\"a\"]\ndef f(m: Mode):\n    pass", true)]
    #[case::annotated_return("Mode = Literal[\"a\"]\ndef f() -> Mode:\n    pass", true)]
    #[case::annotated_variable("Mode = Literal[\"a\"]\nvalue: Mode = x", true)]
    #[case::forward_annotation("def f(m: Mode):\n    pass\nMode = Literal[\"a\"]", true)]
    #[case::pep695_alias_value("Mode = Literal[\"a\"]\ntype Pair = tuple[Mode, Mode]", true)]
    #[case::call_argument("Mode = Literal[\"a\"]\nregister(Mode)", true)]
    #[case::callee("Registry = dict[str, int]\nr = Registry()", true)]
    #[case::attribute_read("int_ = int\nint_.from_bytes(raw)", true)]
    #[case::identity_test("Alias = Generic\nif base is Alias:\n    pass", true)]
    #[case::membership_test("Alias = Undefined\nif x in (Alias, Required):\n    pass", true)]
    #[case::annotation_outranks_runtime(
        "Mode = Literal[\"a\"]\ndef f(m: Mode):\n    pass\nif Mode:\n    pass",
        true
    )]
    #[case::iterated_enum_alias("Colors = ColorEnum\nfor c in Colors:\n    pass", true)]
    #[case::comprehension_over_alias("Colors = ColorEnum\nxs = [c for c in Colors]", true)]
    #[case::equality_against_a_class("Kind = Shape\nif type(x) == Kind:\n    pass", true)]
    #[case::truth_test("flag = registry.enabled\nif flag:\n    pass", false)]
    #[case::negated_test("flag = registry.enabled\nif not flag:\n    pass", false)]
    #[case::while_test("flag = registry.enabled\nwhile flag:\n    pass", false)]
    #[case::ternary_test("flag = registry.enabled\nx = 1 if flag else 2", false)]
    #[case::bool_op("flag = registry.enabled\nx = flag and other", false)]
    #[case::order_comparison("limit = config.limit\nif count > limit:\n    pass", false)]
    #[case::subset_comparison("modes = os.modes\nif {read, write} <= modes:\n    pass", false)]
    #[case::arithmetic("base = config.base\nx = base + 1", false)]
    fn the_read_context_settles_a_value_the_shape_cannot(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(first_is_type_alias(src), expected, "{src}");
    }
}
