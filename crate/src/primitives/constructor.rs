//! Generated-constructor detection. A class header naming a
//! field-ordered constructor generator turns the body's annotated field
//! run into that constructor's positional-or-keyword parameters, so the
//! rules that reorder class members hold the run in source order.

use ruff_python_ast::{Arguments, Decorator, Stmt, StmtClassDef};
use ruff_text_size::{Ranged, TextSize};

use crate::primitives::binding::{decorator_simple_name, tail_identifier, type_head_identifier};

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
        .find(|stmt| is_kw_only_sentinel(stmt))
        .map_or(class.end(), Ranged::end)
}

/// True when a generator's argument list carries an exact
/// `kw_only=True`, the form that makes every generated parameter
/// keyword-only. A non-literal value leaves the run pinned.
fn declares_kw_only(arguments: Option<&Arguments>) -> bool {
    arguments
        .and_then(|args| args.find_keyword("kw_only"))
        .and_then(|kw| kw.value.as_boolean_literal_expr())
        .is_some_and(|literal| literal.value)
}

/// The argument list a decorator carries when applied as a call, `None`
/// where it is applied bare.
fn decorator_arguments(decorator: &Decorator) -> Option<&Arguments> {
    decorator
        .expression
        .as_call_expr()
        .map(|call| &call.arguments)
}

/// True when the class header names a constructor generator that binds
/// the annotated field run by position, named by either a base class or
/// a decorator. The decorator roster mirrors the `attrs` and
/// `dataclasses` entry points `ruff_linter` recognizes, which the
/// `pydantic.dataclasses` re-export shares on its tail segment.
/// Resolution reads that tail segment alone, so an aliased or
/// dotted import matches alike and a same-named local shadow pins a run
/// that would otherwise sort.
fn generates_positional_init(class: &StmtClassDef) -> bool {
    let header = class.arguments.as_deref();
    let based = header.is_some_and(|arguments| {
        arguments.args.iter().any(|base| {
            type_head_identifier(base).is_some_and(|name| matches!(name, "NamedTuple" | "Struct"))
        })
    });
    if based && !declares_kw_only(header) {
        return true;
    }
    class.decorator_list.iter().any(|decorator| {
        decorator_simple_name(decorator).is_some_and(|name| {
            matches!(
                name,
                "attributes" | "attrs" | "dataclass" | "define" | "frozen" | "mutable" | "s"
            )
        }) && !declares_kw_only(decorator_arguments(decorator))
    })
}

/// True when a statement is the `dataclasses.KW_ONLY` sentinel, the
/// pseudo-field whose annotation makes every field below it
/// keyword-only.
fn is_kw_only_sentinel(stmt: &Stmt) -> bool {
    stmt.as_ann_assign_stmt()
        .and_then(|ann| tail_identifier(&ann.annotation))
        == Some("KW_ONLY")
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
