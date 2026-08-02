//! Generated-constructor detection. A class header naming a
//! field-ordered constructor generator turns the body's annotated field
//! run into that constructor's positional-or-keyword parameters.

use ruff_python_ast::{Stmt, StmtAnnAssign, StmtClassDef, helpers::any_over_expr};
use ruff_text_size::{Ranged, TextSize};

use crate::primitives::{
    binding::{ann_assign_with_named_field, is_classvar, tail_identifier, type_head_identifier},
    decorator::{declares_true, decorator_arguments, decorator_simple_name},
};

/// The sort key of a class-body statement read as one of the parameters
/// a generated constructor takes: `0` for a required field and `1` for
/// one carrying a default, then the field name. `None` for a statement
/// the generated signature omits, covering any non-field statement, a
/// `ClassVar` declaration, and the `KW_ONLY` sentinel.
pub(crate) fn classify_field(stmt: &Stmt) -> Option<(u8, &str)> {
    let (ann, name) = ann_assign_with_named_field(stmt)?;
    if is_classvar(&ann.annotation) || is_kw_only_sentinel(ann) {
        return None;
    }
    Some((u8::from(has_default(ann)), name))
}

/// The offset from which a class's annotated fields bind by name rather
/// than by position. A field starting below it holds its source slot.
/// Returns the class start where the header names no field-ordered
/// generator or declares `kw_only=True`, the offset past a `KW_ONLY`
/// sentinel where one splits the run, and the class end where the whole
/// run binds by position.
pub(crate) fn keyword_field_start(class: &StmtClassDef) -> TextSize {
    if !generates_positional_init(class) {
        return class.start();
    }
    class
        .body
        .iter()
        .find(|stmt| stmt.as_ann_assign_stmt().is_some_and(is_kw_only_sentinel))
        .map_or(class.end(), Ranged::end)
}

/// True when the class header names a constructor generator that binds
/// the annotated field run by position, named by either a base class or
/// a decorator. Each name resolves on its tail segment, so a dotted or
/// aliased import matches alike and a same-named local shadow pins a
/// run that would otherwise sort.
fn generates_positional_init(class: &StmtClassDef) -> bool {
    let based = class
        .bases()
        .iter()
        .any(|base| matches!(type_head_identifier(base), Some("NamedTuple" | "Struct")));
    (based && !declares_true(class.arguments.as_deref(), "kw_only"))
        || class.decorator_list.iter().any(|decorator| {
            matches!(
                decorator_simple_name(decorator),
                Some("attributes" | "attrs" | "dataclass" | "define" | "frozen" | "mutable" | "s")
            ) && !declares_true(decorator_arguments(decorator), "kw_only")
        })
}

/// True when an annotated assignment carries a default, either
/// directly via `= value` or through any nested `Call` in the
/// annotation that carries a `default` or `default_factory` keyword.
fn has_default(ann: &StmtAnnAssign) -> bool {
    ann.value.is_some()
        || any_over_expr(&ann.annotation, |e| {
            e.as_call_expr().is_some_and(|c| {
                c.arguments
                    .keywords
                    .iter()
                    .any(|kw| matches!(kw.arg.as_deref(), Some("default" | "default_factory")))
            })
        })
}

/// True when an annotated assignment is the `dataclasses.KW_ONLY`
/// sentinel, the pseudo-field whose annotation makes every field below
/// it keyword-only.
fn is_kw_only_sentinel(ann: &StmtAnnAssign) -> bool {
    tail_identifier(&ann.annotation) == Some("KW_ONLY")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, parse};

    /// Where `keyword_field_start` lands relative to the class body,
    /// reported as the count of fields the offset pins.
    fn pinned_field_count(src: &str) -> usize {
        let source = parse(src);
        let class = first_class(&source);
        let start = keyword_field_start(class);
        class
            .body
            .iter()
            .filter(|stmt| stmt.start() < start)
            .count()
    }

    #[rstest]
    #[case("x: int", Some((0, "x")))]
    #[case("x: int = 1", Some((1, "x")))]
    #[case("x: Annotated[int, Field(default=1)]", Some((1, "x")))]
    #[case("x: Annotated[list, Field(default_factory=list)]", Some((1, "x")))]
    #[case("x: ClassVar[int] = 1", None)]
    #[case("_: KW_ONLY", None)]
    #[case("X = 1", None)]
    #[case("self.x: int = 1", None)]
    fn classify_field_keys_required_before_defaulted(
        #[case] src: &str,
        #[case] expected: Option<(u8, &str)>,
    ) {
        let source = parse(src);
        assert_eq!(classify_field(&source.ast().body[0]), expected);
    }

    #[test]
    fn keyword_field_start_pins_a_sentinel_free_generated_class_whole() {
        let src = "@dataclass\nclass C:\n    a: int\n    b: int\n";
        assert_eq!(pinned_field_count(src), 2);
    }

    #[rstest]
    #[case("class C(NamedTuple):\n    b: int\n    a: int\n", 2)]
    #[case("class C(typing.NamedTuple):\n    b: int\n    a: int\n", 2)]
    #[case("class C(msgspec.Struct):\n    b: int\n    a: int\n", 2)]
    #[case("class C(Struct[int]):\n    b: int\n    a: int\n", 2)]
    #[case("class C(namedtuple(\"P\", \"x y\")):\n    b: int\n    a: int\n", 0)]
    #[case("class C(TypedDict):\n    b: int\n    a: int\n", 0)]
    #[case("class C(BaseModel):\n    b: int\n    a: int\n", 0)]
    #[case("class C:\n    b: int\n    a: int\n", 0)]
    fn keyword_field_start_reads_the_base_list(#[case] src: &str, #[case] expected: usize) {
        assert_eq!(pinned_field_count(src), expected);
    }

    #[rstest]
    #[case("@dataclass\nclass C:\n    b: int\n", 1)]
    #[case("@dataclasses.dataclass\nclass C:\n    b: int\n", 1)]
    #[case("@pydantic.dataclasses.dataclass\nclass C:\n    b: int\n", 1)]
    #[case("@dataclass(frozen=True)\nclass C:\n    b: int\n", 1)]
    #[case("@attr.s\nclass C:\n    b: int\n", 1)]
    #[case("@attr.attrs\nclass C:\n    b: int\n", 1)]
    #[case("@attr.attributes\nclass C:\n    b: int\n", 1)]
    #[case("@attrs.define\nclass C:\n    b: int\n", 1)]
    #[case("@attrs.frozen\nclass C:\n    b: int\n", 1)]
    #[case("@attrs.mutable\nclass C:\n    b: int\n", 1)]
    #[case("@register\nclass C:\n    b: int\n", 0)]
    fn keyword_field_start_reads_the_decorator_list(#[case] src: &str, #[case] expected: usize) {
        assert_eq!(pinned_field_count(src), expected);
    }

    #[test]
    fn keyword_field_start_stops_at_a_kw_only_sentinel() {
        let src = "@dataclass\nclass C:\n    a: int\n    _: KW_ONLY\n    c: int\n    b: int\n";
        assert_eq!(pinned_field_count(src), 2, "the field and the sentinel pin");
    }

    #[rstest]
    #[case("@dataclass(kw_only=True)\nclass C:\n    b: int\n", 0)]
    #[case("@attrs.define(kw_only=True)\nclass C:\n    b: int\n", 0)]
    #[case("class C(Struct, kw_only=True):\n    b: int\n", 0)]
    #[case("@dataclass()\nclass C:\n    b: int\n", 1)]
    #[case("@dataclass(kw_only=False)\nclass C:\n    b: int\n", 1)]
    #[case("@dataclass(kw_only=FLAG)\nclass C:\n    b: int\n", 1)]
    #[case("@dataclass(kw_only=1)\nclass C:\n    b: int\n", 1)]
    fn keyword_field_start_unlocks_only_on_a_literal_true(
        #[case] src: &str,
        #[case] expected: usize,
    ) {
        assert_eq!(pinned_field_count(src), expected);
    }
}
