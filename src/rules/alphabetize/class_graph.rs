//! Class-scope assignment tiering. Sorts the constant family (bare
//! `NAME = value` and `ClassVar`-annotated assignments) and the
//! data-field family (other single-name annotated assignments) through
//! one shared dependency graph, so a member never sorts above a sibling,
//! or below a definition, that reads it at class-definition time. Each
//! family still redistributes only across the slots it already holds.
//! Reverts the reorder on a duplicate name, a reference cycle, or an
//! assembled order that would seat a referent after a reader.

use std::collections::HashMap;

use ruff_python_ast::{Expr, Stmt, helpers::map_subscript};
use ruff_text_size::Ranged;

use super::{
    ann_assign_with_named_field, has_default, simple_name_assign,
    tiering::{def_run_tier_keys, eval_time_refs},
};
use crate::primitives::{
    binding::tail_identifier,
    orderer::{permute_full, slot_positions},
};

/// Classifies a class-body statement as a single-name assignment,
/// returning its target name and whether it is a constant (`true`) or a
/// data field (`false`). A bare assignment and a `ClassVar`-annotated
/// assignment are constants, every other single-name annotated
/// assignment is a field. `None` for any other statement.
fn class_assign_member(stmt: &Stmt) -> Option<(&str, bool)> {
    match ann_assign_with_named_field(stmt) {
        Some((ann, name)) => Some((name, is_classvar(&ann.annotation))),
        None => simple_name_assign(stmt).map(|name| (name, true)),
    }
}

/// True when an annotation's outermost head is `ClassVar`, covering the
/// bare `ClassVar` and the subscripted `ClassVar[...]` forms named
/// directly or through a module attribute (`typing.ClassVar`).
fn is_classvar(annotation: &Expr) -> bool {
    tail_identifier(map_subscript(annotation)) == Some("ClassVar")
}

/// True when every reader in `body` keeps each assignment member it
/// names ahead of itself in `order`. A reader is an assignment member or
/// a class or function definition, the latter naming a constant through
/// its decorators, base classes, or parameter defaults.
fn order_is_sound(
    order: &[usize],
    body: &[Stmt],
    member_at: &HashMap<&str, usize>,
    defer_annotations: bool,
) -> bool {
    let position = slot_positions(order);
    body.iter().enumerate().all(|(reader, stmt)| {
        let reads_eval_time = class_assign_member(stmt).is_some()
            || matches!(stmt, Stmt::ClassDef(_) | Stmt::FunctionDef(_));
        if !reads_eval_time {
            return true;
        }
        eval_time_refs(stmt, defer_annotations).iter().all(|name| {
            member_at
                .get(name)
                .is_none_or(|&referent| referent == reader || position[referent] < position[reader])
        })
    })
}

/// Sorts the class body's constant and data-field families through one
/// tiered dependency graph, rewriting `order` in place. Leaves `order`
/// untouched when fewer than two members reorder, a name repeats, the
/// reference graph cycles, or the sorted order would strand a reader.
pub(super) fn permute_class_assigns(order: &mut [usize], body: &[Stmt], defer_annotations: bool) {
    let Some(tier_keys) = def_run_tier_keys(body, defer_annotations, |stmt| {
        class_assign_member(stmt).map(|(name, _)| (name, name))
    }) else {
        return;
    };
    if tier_keys.len() < 2 {
        return;
    }
    let member_at: HashMap<&str, usize> = body
        .iter()
        .enumerate()
        .filter_map(|(idx, stmt)| class_assign_member(stmt).map(|(name, _)| (name, idx)))
        .collect();
    let snapshot = order.to_vec();
    permute_full(order, body, |stmt| {
        let (ann, _) = ann_assign_with_named_field(stmt)?;
        if is_classvar(&ann.annotation) {
            return None;
        }
        let (tier, name) = tier_keys[&stmt.range().start()];
        Some((tier, u8::from(has_default(ann)), name))
    });
    permute_full(order, body, |stmt| {
        class_assign_member(stmt)
            .filter(|&(_, is_const)| is_const)
            .map(|_| tier_keys[&stmt.range().start()])
    });
    if !order_is_sound(order, body, &member_at, defer_annotations) {
        order.copy_from_slice(&snapshot);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_class, parse};

    fn class_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let body = &first_class(&source).body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        permute_class_assigns(&mut order, body, false);
        order
    }

    #[rstest]
    #[case("x: ClassVar = 1", true)]
    #[case("x: ClassVar[int] = 1", true)]
    #[case("x: typing.ClassVar[int] = 1", true)]
    #[case("x: int = 1", false)]
    #[case("x: list[int] = []", false)]
    #[case("x: Final[int] = 1", false)]
    fn is_classvar_keys_off_the_annotation_head(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let ann = source.ast().body[0]
            .as_ann_assign_stmt()
            .expect("annotated assignment");
        assert_eq!(is_classvar(&ann.annotation), expected);
    }

    #[rstest]
    #[case("X = 1", Some(("X", true)))]
    #[case("X: ClassVar[int] = 1", Some(("X", true)))]
    #[case("x: int = 1", Some(("x", false)))]
    #[case("x, y = 1, 2", None)]
    #[case("self.x = 1", None)]
    fn class_assign_member_routes_constants_and_fields(
        #[case] src: &str,
        #[case] expected: Option<(&str, bool)>,
    ) {
        let source = parse(src);
        assert_eq!(class_assign_member(&source.ast().body[0]), expected);
    }

    #[test]
    fn sorts_fields_below_a_derived_constant() {
        let order =
            class_order("class C:\n    width: int = 10\n    height: int = 20\n    HALF = width\n");
        assert_eq!(order, vec![1, 0, 2], "fields sort and HALF stays sound");
    }

    #[test]
    fn routes_a_classvar_among_the_bare_constants() {
        let order = class_order(
            "class C:\n    TIMEOUT = 30\n    RETRIES: ClassVar[int] = 3\n    host: str\n    port: int\n",
        );
        assert_eq!(order, vec![1, 0, 2, 3], "RETRIES sorts ahead of TIMEOUT");
    }

    #[test]
    fn reverts_when_a_field_sort_strands_an_interleaved_reader() {
        let order =
            class_order("class C:\n    width: int = 10\n    HALF = width\n    height: int = 20\n");
        assert_eq!(
            order,
            vec![0, 1, 2],
            "the interleaved reader holds source order"
        );
    }

    #[test]
    fn reverts_when_a_method_default_strands_a_constant() {
        let order = class_order(
            "class C:\n    SCALE = 2\n    def render(self, factor=SCALE): ...\n    APPLE = 1\n",
        );
        assert_eq!(
            order,
            vec![0, 1, 2],
            "SCALE may not sort below the method reading it"
        );
    }

    #[test]
    fn declines_a_cross_family_cycle() {
        let order = class_order("class C:\n    A: int = B\n    B = A\n");
        assert_eq!(order, vec![0, 1], "a cross-family cycle keeps source order");
    }
}
