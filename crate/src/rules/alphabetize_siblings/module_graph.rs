//! Module-scope definition banding. Sorts the classes and the functions
//! of a section as one run keyed by band, tier, method group, and name,
//! the classes seating above the functions, each band tiering through
//! the dependency graph the whole run shares, and the function band
//! keeping the method-group order.

use std::ops::Range;

use ruff_python_ast::Stmt;

use super::members::function_key;
use crate::primitives::tiering::{Evaluation, permute_defs};

/// The band a module-level definition sorts into, a class ahead of a
/// function.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Band {
    Class,
    Function,
}

/// Sorts a section's module-level definitions through one tiered
/// dependency graph, rewriting `order` in place. A member `holds`
/// selects keeps its source slot, and a member the permutation would
/// seat across a binding it evaluates holds its slot while the rest of
/// the run still sorts. Leaves `order` untouched when a name repeats or
/// the reference graph cycles.
pub(super) fn permute_module_defs<'src>(
    order: &mut [usize],
    body: &'src [Stmt],
    range: Range<usize>,
    evaluation: Evaluation<'_, 'src>,
    holds: impl Fn(&'src Stmt) -> bool,
    group_methods: bool,
) {
    permute_defs(
        order,
        body,
        range,
        evaluation,
        holds,
        |stmt| banded_member(stmt, group_methods),
        |tier, (band, group, name)| (band, tier, group, name),
    );
}

/// The name a module-level definition binds beside its band, method
/// group, and name, `None` for any other statement. A class takes the
/// class band and a function the function band, grouped as a method is
/// while `group_methods`.
fn banded_member(stmt: &Stmt, group_methods: bool) -> Option<(&str, (Band, u8, &str))> {
    match stmt {
        Stmt::ClassDef(class) => {
            let name = class.name.as_str();
            Some((name, (Band::Class, 0, name)))
        }
        Stmt::FunctionDef(func) => {
            let (group, name) = function_key(func, group_methods);
            Some((name, (Band::Function, group, name)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::testing::{evaluated, parse};

    /// The new-order permutation `permute_module_defs` produces over
    /// `src`, holding nothing and grouping the functions.
    fn module_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let evaluated = evaluated(&source, body);
        permute_module_defs(
            &mut order,
            body,
            0..body.len(),
            evaluated.evaluation(),
            |_| false,
            true,
        );
        order
    }

    #[test]
    fn permute_module_defs_declines_a_reference_cycle() {
        let src = indoc! {"
            class Zed(Alpha):
                pass

            def render():
                pass

            class Alpha(Zed):
                pass
        "};
        assert_eq!(module_order(src), vec![0, 1, 2]);
    }

    #[test]
    fn permute_module_defs_holds_a_class_reading_a_function_as_it_binds() {
        let src = indoc! {"
            def make_base():
                return object

            class Zed(make_base()):
                pass

            def alpha():
                pass
        "};
        assert_eq!(
            module_order(src),
            vec![0, 1, 2],
            "Zed evaluates make_base where it binds, so neither crosses the other"
        );
    }

    #[test]
    fn permute_module_defs_holds_a_function_a_subscripted_base_reaches() {
        let src = indoc! {"
            class Generic:
                def __class_getitem__(cls, item):
                    return zzz_dispatch(item)

            def zzz_dispatch(item):
                return object

            class Widget(Generic[str]):
                pass
        "};
        assert_eq!(
            module_order(src),
            vec![0, 1, 2],
            "subscripting Generic runs its body, which reads zzz_dispatch, so it holds above"
        );
    }

    #[test]
    fn permute_module_defs_settles_a_subscripted_base_across_repeat_passes() {
        let src = indoc! {"
            class Generic:
                def __class_getitem__(cls, item):
                    return zzz_dispatch(item)

            def zzz_dispatch(item):
                return object

            class Widget(Generic[str]):
                pass
        "};
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let evaluated = evaluated(&source, body);
        for pass in 0..3 {
            permute_module_defs(
                &mut order,
                body,
                0..body.len(),
                evaluated.evaluation(),
                |_| false,
                true,
            );
            assert_eq!(order, vec![0, 1, 2], "pass {pass} strands zzz_dispatch");
        }
    }

    #[test]
    fn permute_module_defs_holds_every_definition_under_an_opaque_base() {
        let src = indoc! {"
            from vendor import Generic

            def zzz_dispatch(item):
                return object

            class Widget(Generic[str]):
                pass
        "};
        assert_eq!(
            module_order(src),
            vec![0, 1, 2],
            "subscripting an imported base runs code reaching any binding, so nothing crosses it"
        );
    }

    #[test]
    fn permute_module_defs_sorts_past_a_plain_imported_base() {
        let src = indoc! {"
            from vendor import Base

            def zzz_dispatch(item):
                return object

            class Widget(Base):
                pass
        "};
        assert_eq!(
            module_order(src),
            vec![0, 2, 1],
            "a base naming no call runs nothing, so the class still bands above the function"
        );
    }

    #[test]
    fn permute_module_defs_seats_a_derived_class_in_the_class_band() {
        let src = indoc! {"
            def render():
                pass

            class Gadget:
                pass

            def build():
                pass

            class Widget(Gadget):
                pass
        "};
        assert_eq!(
            module_order(src),
            vec![1, 3, 2, 0],
            "Widget tiers behind Gadget inside the class band rather than behind the functions"
        );
    }

    #[test]
    fn permute_module_defs_sorts_the_function_band_by_method_group() {
        let src = indoc! {"
            def Factory():
                pass

            def _helper():
                pass

            def __getattr__(name):
                pass
        "};
        assert_eq!(module_order(src), vec![2, 1, 0]);
    }
}
