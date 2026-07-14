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
//! `name_binds_data` looks the base of a subscript up in the module
//! binding table, so `SETTINGS["db"]` is a lookup once `SETTINGS` is
//! found assigned a dict display in the same file, and `Box[int]` is a
//! type once `Box` is found declared by a `class` statement. An import,
//! a rebind, and a call all leave the base undecided, which stays a
//! type.
//!
//! `is_type_alias` adds `module_reads_as_data`, so a name the module
//! truth-tests, order-compares, or does arithmetic on holds data,
//! whereas one read in an annotation names a type. `band-constants`
//! calls `value_is_alias` and skips this last check, sorting its
//! sub-band on the value alone.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{
    Expr, ExprAttribute, ExprBinOp, ExprList, ExprNumberLiteral, ExprSubscript, ExprTuple,
    ExprUnaryOp, Number, Operator, Stmt, UnaryOp, helpers::is_dunder, name::UnqualifiedName,
};
use ruff_python_stdlib::typing::{is_literal_member, is_pep_593_generic_member};

use super::binding::{BindingAnalysis, BindingKind, ModuleAssignment, module_assignments};

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

    /// The binding table the checks read.
    pub(crate) fn analysis(&self) -> &'src BindingAnalysis {
        self.analysis
    }
}

/// True when `site` binds a type. The value must name an object that
/// already exists and no read of the bound name may prove it holds data.
pub(crate) fn is_type_alias(site: &ModuleAssignment<'_>, ctx: &AliasContext<'_>) -> bool {
    site.value.is_some_and(|value| value_is_alias(value, ctx))
        && !ctx.analysis.module_reads_as_data(&site.target.id)
}

/// True when `value` names an object that already exists rather than
/// constructing a new one. A `|` union holds only when both sides do,
/// leaving `int | float` an alias and `1 | 2` a constant.
pub(crate) fn value_is_alias(value: &Expr, ctx: &AliasContext<'_>) -> bool {
    Resolver {
        ctx,
        visited: HashSet::new(),
    }
    .alias_value(value)
}

/// One classification pass, holding the bases already resolved so a
/// cyclic base settles once.
struct Resolver<'ctx, 'src> {
    ctx: &'ctx AliasContext<'src>,
    visited: HashSet<&'src str>,
}

impl<'src> Resolver<'_, 'src> {
    fn alias_value(&mut self, value: &Expr) -> bool {
        match value {
            Expr::BinOp(ExprBinOp {
                left,
                op: Operator::BitOr,
                right,
                ..
            }) => self.union_arm(left) && self.union_arm(right),
            Expr::Subscript(subscript) => !self.subscript_is_runtime(subscript),
            _ => UnqualifiedName::from_expr(value).is_some_and(|base| self.dotted_is_alias(&base)),
        }
    }

    /// True when a dotted name may hold a type. A dunder anywhere along
    /// the path names a runtime value (`__builtins__`, `sys.__stdout__`),
    /// and a head the module binds to data holds data.
    fn dotted_is_alias(&mut self, base: &UnqualifiedName<'_>) -> bool {
        !base.segments().iter().any(|segment| is_dunder(segment))
            && !base
                .segments()
                .first()
                .copied()
                .is_some_and(|head| self.name_binds_data(head))
    }

    /// True when `arm` is a type a PEP 604 union may carry, an alias
    /// value or the `None` of an optional. A bare `None` outside a union
    /// binds a sentinel rather than naming a type.
    fn union_arm(&mut self, arm: &Expr) -> bool {
        arm.is_none_literal_expr() || self.alias_value(arm)
    }

    /// True when `subscript` indexes a value at runtime rather than
    /// parametrizing a type, proven either by the module binding the base
    /// to data or by a node the typing grammar never admits in the slice.
    fn subscript_is_runtime(&mut self, subscript: &ExprSubscript) -> bool {
        let Some(base) = UnqualifiedName::from_expr(&subscript.value) else {
            // A call or a chained subscript roots the base, as in
            // `get_registry().default` and `registry[0][1]`.
            return true;
        };
        if !self.dotted_is_alias(&base) {
            return true;
        }
        let slice = subscript.slice.as_ref();
        match base.segments().last().copied() {
            // PEP 586 admits an int, a bool, and bytes, and `Literal` is
            // the sole construct that carries them.
            Some(tail) if is_literal_member(tail) => self.runtime_only(slice, true),
            // PEP 593 leaves every argument after the first arbitrary, so
            // `Annotated[int, Field(gt=0)]` reads only its type.
            Some(tail) if is_pep_593_generic_member(tail) => {
                let annotated = slice
                    .as_tuple_expr()
                    .and_then(|tuple| tuple.elts.first())
                    .unwrap_or(slice);
                self.runtime_only(annotated, false)
            }
            _ => self.runtime_only(slice, false),
        }
    }

    /// True when `expr` cannot stand at this position in a type
    /// expression, which proves the enclosing subscript indexes data.
    /// `in_literal` widens the grammar to the arguments a `Literal[...]`
    /// carries.
    ///
    /// The match is exhaustive over `Expr` rather than closing on a
    /// wildcard, so a new AST node raises a compile error instead of
    /// silently reading as a type.
    fn runtime_only(&mut self, expr: &Expr, in_literal: bool) -> bool {
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
                    || self.runtime_only(left, in_literal)
                    || self.runtime_only(right, in_literal)
            }

            // A signed integer stands only inside `Literal`, so
            // `items[-1]` indexes data whereas `Literal[-1]` names a type.
            Expr::UnaryOp(ExprUnaryOp { op, operand, .. }) => {
                !in_literal
                    || !matches!(
                        (op, operand.as_ref()),
                        (
                            UnaryOp::UAdd | UnaryOp::USub,
                            Expr::NumberLiteral(ExprNumberLiteral {
                                value: Number::Int(_),
                                ..
                            })
                        )
                    )
            }

            // An int, a bool, and bytes ride on `Literal` alone.
            Expr::BooleanLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(ExprNumberLiteral {
                value: Number::Int(_),
                ..
            }) => !in_literal,
            // A float and a complex have no place in any type expression.
            Expr::NumberLiteral(_) => true,

            // A dunder names a runtime value, and a name the module binds
            // to data indexes it (`table[idx]` under `idx = 0`).
            Expr::Name(name) => is_dunder(&name.id) || self.name_binds_data(&name.id),
            Expr::Attribute(ExprAttribute { attr, value, .. }) => {
                is_dunder(attr) || self.runtime_only(value, in_literal)
            }

            // A container inherits the proof its members carry.
            Expr::List(ExprList { elts, .. }) | Expr::Tuple(ExprTuple { elts, .. }) => elts
                .iter()
                .any(|element| self.runtime_only(element, in_literal)),
            Expr::Starred(starred) => self.runtime_only(&starred.value, in_literal),
            Expr::Subscript(subscript) => self.subscript_is_runtime(subscript),

            // A string is a forward reference, `None` is a type, and `...`
            // is the shape of `Callable[..., int]`.
            Expr::EllipsisLiteral(_) | Expr::NoneLiteral(_) | Expr::StringLiteral(_) => false,
        }
    }

    /// True when this module binds `name` to data it builds. Anything the
    /// module leaves undecided reads as false, in that only a value built
    /// here proves the name holds data.
    fn name_binds_data(&mut self, name: &str) -> bool {
        let analysis = self.ctx.analysis;
        let kinds = analysis.module_binding_kinds(name);
        // An unbound name is a builtin, a star-import, or a global
        // injected at runtime. A `class` statement binds a type. An
        // import and a rebind are decided elsewhere. None says data.
        if kinds.is_empty()
            || kinds.contains(&BindingKind::ClassDef)
            || kinds.contains(&BindingKind::Import)
            || analysis.module_reassigned(name)
        {
            return false;
        }
        let Some((&key, &value)) = self.ctx.values.get_key_value(name) else {
            return false;
        };
        // A cycle, as in `A = A[0]`.
        if !self.visited.insert(key) {
            return false;
        }
        // A call binds a type as readily as data (`T = TypeVar("T")`),
        // and an attribute is undecidable in the module alone
        // (`opener = TarFile.open` against `path_sep = os.sep`).
        !matches!(value, Expr::Attribute(_) | Expr::Call(_)) && !self.alias_value(value)
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
    #[case::negative_index("X = items[-1]", false)]
    #[case::positive_signed_index("X = offsets[+1]", false)]
    #[case::inverted_index("X = mask[~3]", false)]
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
    #[case::rebound_dict_base("A = {}\nA = {\"k\": 1}\nX = A[\"k\"]", true)]
    #[case::attribute_valued_base("sep = os.sep\nX = table[sep]", true)]
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
