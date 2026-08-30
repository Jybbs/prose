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

use ruff_python_ast::{
    Expr, ExprAttribute, ExprBinOp, ExprList, ExprNumberLiteral, ExprSubscript, ExprTuple,
    ExprUnaryOp, Number, Operator, Stmt, UnaryOp, helpers::is_dunder, name::UnqualifiedName,
};
use ruff_python_stdlib::typing::{is_literal_member, is_pep_593_generic_member};
use rustc_hash::{FxHashMap, FxHashSet};

use super::binding::{BindingAnalysis, BindingKind, ModuleAssignment, module_assignments};

mod resolver;

use resolver::Resolver;

/// The module-scope evidence the alias read consumes, pairing the
/// binding table with every module-scope assignment and an index from
/// each bound name to its value.
pub(crate) struct AliasContext<'src> {
    analysis: &'src BindingAnalysis,
    sites: Vec<ModuleAssignment<'src>>,
    values: FxHashMap<&'src str, &'src Expr>,
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

    /// The binding table the checks read.
    pub(crate) fn analysis(&self) -> &'src BindingAnalysis {
        self.analysis
    }

    /// Every module-scope single-name assignment, in source order.
    pub(crate) fn sites(&self) -> &[ModuleAssignment<'src>] {
        &self.sites
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
        visited: FxHashSet::default(),
    }
    .alias_value(value)
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

    /// Classifies the first module-scope assignment, reading how the
    /// rest of the module uses the name it binds.
    fn first_is_type_alias(src: &str) -> bool {
        let source = parse(src);
        let analysis = source.binding_analysis();
        let ctx = AliasContext::new(&source.ast().body, analysis);
        is_type_alias(ctx.sites().first().expect("a module assignment"), &ctx)
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
}
