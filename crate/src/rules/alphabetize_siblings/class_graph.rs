//! Class-scope assignment tiering. Sorts the constant family (bare
//! `NAME = value` and `ClassVar`-annotated assignments) and the
//! data-field family (other single-name annotated assignments) through
//! one shared dependency graph, so a member never sorts above a sibling
//! or below a statement that reads it at class-definition time. Each
//! family still redistributes only across the slots it already holds.
//! A field bound by position in a generated constructor holds its slot
//! while the constants around it still sort.

use std::ops::Range;

use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};

use crate::primitives::{
    binding::{ann_assign_with_named_field, is_classvar, single_name_target},
    constructor::classify_field,
    orderer::permute_in_place,
    tiering::{CallReach, def_run_tier_keys, permute_or_repair},
};

/// Sorts a section's constant and data-field families through one tiered
/// dependency graph, rewriting `order` in place. Tiering and the
/// soundness check scope to `range`, so a marker-divided section sorts on
/// its own. A field starting below `keyword_fields_from` holds its slot
/// while the constants around it still sort. Leaves `order` untouched
/// when fewer than two members reorder, a name repeats, the reference
/// graph cycles, or the sorted order would strand a reader.
pub(super) fn permute_class_assigns<'src>(
    order: &mut [usize],
    body: &'src [Stmt],
    range: Range<usize>,
    defer_annotations: bool,
    keyword_fields_from: TextSize,
    reachable: &CallReach<'src>,
) {
    let Some(tier_keys) = def_run_tier_keys(&body[range.clone()], defer_annotations, |stmt| {
        class_assign_member(stmt).map(|(name, _)| (name, name))
    }) else {
        return;
    };
    if tier_keys.len() < 2 {
        return;
    }
    permute_or_repair(
        order,
        body,
        &range,
        defer_annotations,
        reachable,
        |stmt| class_assign_member(stmt).map(|(name, _)| name),
        |order, pinned| {
            let fields_moved = permute_in_place(order, body, range.clone(), |stmt| {
                if stmt.start() < keyword_fields_from || pinned.contains(&stmt.start()) {
                    return None;
                }
                let (default, _) = classify_field(stmt)?;
                let (tier, name) = tier_keys[&stmt.start()];
                Some((tier, default, name))
            });
            let constants_moved = permute_in_place(order, body, range.clone(), |stmt| {
                class_assign_member(stmt)
                    .filter(|&(_, is_const)| is_const && !pinned.contains(&stmt.start()))
                    .map(|_| tier_keys[&stmt.start()])
            });
            fields_moved || constants_moved
        },
    );
}

/// Classifies a class-body statement as a single-name assignment,
/// returning its target name and whether it is a constant (`true`) or a
/// data field (`false`). A bare assignment and a `ClassVar`-annotated
/// assignment are constants, every other single-name annotated
/// assignment is a field. `None` for any other statement.
fn class_assign_member(stmt: &Stmt) -> Option<(&str, bool)> {
    match ann_assign_with_named_field(stmt) {
        Some((ann, name)) => Some((name, is_classvar(&ann.annotation))),
        None => single_name_target(stmt.as_assign_stmt()?).map(|name| (name, true)),
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::{
        primitives::{constructor::keyword_field_start, tiering::call_reachable},
        testing::{first_class, parse},
    };

    fn class_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let class = first_class(&source);
        let body = &class.body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let reachable = call_reachable(body);
        permute_class_assigns(
            &mut order,
            body,
            0..body.len(),
            false,
            keyword_field_start(class),
            &reachable,
        );
        order
    }

    #[rstest]
    #[case("X = 1", Some(("X", true)))]
    #[case("X: ClassVar[int] = 1", Some(("X", true)))]
    #[case("x: int = 1", Some(("x", false)))]
    #[case("x, y = 1, 2", None)]
    #[case("self.x = 1", None)]
    #[case("self.x: int = 1", None)]
    fn class_assign_member_routes_constants_and_fields(
        #[case] src: &str,
        #[case] expected: Option<(&str, bool)>,
    ) {
        let source = parse(src);
        assert_eq!(class_assign_member(&source.ast().body[0]), expected);
    }

    #[test]
    fn declines_a_cross_family_cycle() {
        let order = class_order("class C:\n    A: int = B\n    B = A\n");
        assert_eq!(order, vec![0, 1], "a cross-family cycle keeps source order");
    }

    #[test]
    fn holds_a_generated_constructors_field_run() {
        let order = class_order("@dataclass\nclass C:\n    width: int\n    height: int\n");
        assert_eq!(order, vec![0, 1], "the field run holds its source order");
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
    fn routes_a_classvar_among_the_bare_constants() {
        let order = class_order(indoc! {"
            class C:
                TIMEOUT = 30
                RETRIES: ClassVar[int] = 3
                host: str
                port: int
        "});
        assert_eq!(order, vec![1, 0, 2, 3], "RETRIES sorts ahead of TIMEOUT");
    }

    #[test]
    fn sorts_constants_around_a_pinned_field_run() {
        let order = class_order(
            "@dataclass\nclass C:\n    ZEBRA = 1\n    APPLE = 2\n    width: int\n    height: int\n",
        );
        assert_eq!(order, vec![1, 0, 2, 3], "constants sort, fields hold");
    }

    #[test]
    fn sorts_fields_below_a_derived_constant() {
        let order =
            class_order("class C:\n    width: int = 10\n    height: int = 20\n    HALF = width\n");
        assert_eq!(order, vec![1, 0, 2], "fields sort and HALF stays sound");
    }

    #[test]
    fn sorts_only_the_fields_below_a_kw_only_sentinel() {
        let order = class_order(indoc! {"
            @dataclass
            class C:
                width: int
                _: KW_ONLY
                zebra: int
                apple: int
        "});
        assert_eq!(order, vec![0, 1, 3, 2], "the sentinel splits the run");
    }
}
