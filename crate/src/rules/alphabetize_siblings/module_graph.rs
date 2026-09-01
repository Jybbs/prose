//! Module-scope definition banding. Sorts the classes and the functions
//! of a section as one run keyed by band, tier, method group, and name,
//! the classes seating above the functions, each band tiering through
//! the dependency graph the whole run shares, and the function band
//! keeping the method-group order.

use std::ops::Range;

use ruff_python_ast::Stmt;

use super::members::function_key;
use crate::primitives::tiering::{DefRun, Evaluation};

/// The band a module-level definition sorts into, a class ahead of a
/// function.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Band {
    Class,
    Function,
}

/// A section's module-level definitions, tiered as one run.
pub(super) type ModuleDefs<'a, 'src> = DefRun<'a, 'src, (Band, u8, &'src str)>;

/// Prepares a section's module-level definitions as one tiered run, so a
/// caller permuting it on every pass of a fixed-point loop tiers it once.
/// `None` where a name repeats or the reference graph cycles.
pub(super) fn module_def_run<'a, 'src>(
    body: &'src [Stmt],
    range: Range<usize>,
    evaluation: Evaluation<'a, 'src>,
    group_methods: bool,
) -> Option<ModuleDefs<'a, 'src>> {
    DefRun::of(body, range, evaluation, |stmt| {
        banded_member(stmt, group_methods)
    })
}

/// Permutes a prepared module-definition run, rewriting `order` in place
/// with the classes seating above the functions and each band tiering
/// through the graph the whole run shares. A member `holds` selects keeps
/// its source slot, and a member the permutation would seat across a
/// binding it evaluates holds its slot while the rest of the run sorts.
pub(super) fn permute_module_run<'src>(
    run: &ModuleDefs<'_, 'src>,
    order: &mut [usize],
    body: &'src [Stmt],
    holds: impl Fn(&'src Stmt) -> bool,
) {
    run.permute(order, body, holds, |tier, (band, group, name)| {
        (band, tier, group, name)
    });
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

    /// The new-order permutation a prepared module run produces over
    /// `src`, holding nothing and grouping the functions.
    fn module_order(src: &str) -> Vec<usize> {
        let source = parse(src);
        let body = &source.ast().body;
        let mut order: Vec<usize> = (0..body.len()).collect();
        let evaluated = evaluated(&source, body);
        if let Some(run) = module_def_run(body, 0..body.len(), evaluated.evaluation(), true) {
            permute_module_run(&run, &mut order, body, |_| false);
        }
        order
    }

    #[test]
    fn module_defs_declines_a_reference_cycle() {
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
    fn module_defs_holds_a_class_reading_a_function_as_it_binds() {
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
    fn module_defs_holds_a_function_a_subscripted_base_reaches() {
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
    fn module_defs_seats_a_derived_class_in_the_class_band() {
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
    fn module_defs_settles_a_subscripted_base_across_repeat_passes() {
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
            let run = module_def_run(body, 0..body.len(), evaluated.evaluation(), true)
                .expect("the run tiers");
            permute_module_run(&run, &mut order, body, |_| false);
            assert_eq!(order, vec![0, 1, 2], "pass {pass} strands zzz_dispatch");
        }
    }

    #[test]
    fn module_defs_sorts_past_a_plain_imported_base() {
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
    fn module_defs_sorts_the_function_band_by_method_group() {
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
