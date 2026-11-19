//! The dict literals whose every use is a lookup, the only literals
//! `alphabetize-siblings` sorts. A literal used in place is a lookup as
//! `{...}[k]`, `{...}.get`, `k in {...}`, or `k not in {...}`. A
//! literal bound once to a name through `name = {...}` or
//! `name: T = {...}` sorts where each read of that binding in the file
//! is a subscript load, store, or delete, `.get`, `in` or `not in`,
//! `len`, or `sorted`. A binding in a class body, one `__all__` lists,
//! one a `nonlocal` or `del` statement names, and one written again,
//! through `global` included, keep the order written, as every other
//! literal does.

use ruff_python_ast::{AnyNodeRef, CmpOp, Expr, Stmt, visitor::source_order::TraversalSignal};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    primitives::{
        binding::{BindingAnalysis, single_name_assignment},
        dunder_all::DunderAll,
        walk::{ParentedProbe, filter_map_over_stmts, walk_parented_exprs},
    },
    source::Source,
};

/// The subjects of every lookup a walk reaches, a name as its offset and
/// a dict literal used in place as its range.
struct Lookups<'a> {
    bindings: &'a BindingAnalysis,
    in_place: FxHashSet<TextRange>,
    names: FxHashSet<TextSize>,
}

impl Lookups<'_> {
    /// True where `parent`, under `ancestors`, looks a key up in
    /// `subject` or reads the size or sorted keys of a name.
    fn looks_up(&self, subject: &Expr, parent: AnyNodeRef, ancestors: &[AnyNodeRef]) -> bool {
        match parent {
            AnyNodeRef::Arguments(arguments) => {
                subject.is_name_expr()
                    && matches!(&*arguments.args, [only] if only.range() == subject.range())
                    && ancestors
                        .iter()
                        .rev()
                        .nth(1)
                        .and_then(|call| call.as_expr_call()?.func.as_name_expr())
                        .is_some_and(|func| {
                            matches!(func.id.as_str(), "len" | "sorted")
                                && !self.bindings.binds_name(&func.id)
                        })
            }
            AnyNodeRef::ExprAttribute(attribute) => attribute.attr.as_str() == "get",
            AnyNodeRef::ExprCompare(compare) => {
                compare
                    .ops
                    .iter()
                    .zip(compare.comparators())
                    .any(|(op, right)| {
                        matches!(op, CmpOp::In | CmpOp::NotIn) && right.range() == subject.range()
                    })
            }
            AnyNodeRef::ExprSubscript(subscript) => subscript.value.range() == subject.range(),
            _ => false,
        }
    }
}

impl<'a> ParentedProbe<'a> for Lookups<'_> {
    fn probe(
        &mut self,
        expr: &'a Expr,
        parent: AnyNodeRef<'a>,
        ancestors: &[AnyNodeRef<'a>],
    ) -> TraversalSignal {
        match expr {
            Expr::Dict(dict) if self.looks_up(expr, parent, ancestors) => {
                self.in_place.insert(dict.range());
            }
            Expr::Name(name) if self.looks_up(expr, parent, ancestors) => {
                self.names.insert(name.start());
            }
            _ => {}
        }
        TraversalSignal::Traverse
    }
}

/// The ranges of the dict literals in `source` whose every use is a
/// lookup, per the module doc.
pub(crate) fn lookup_dicts(source: &Source) -> FxHashSet<TextRange> {
    let bindings = source.binding_analysis();
    let mut lookups = Lookups {
        bindings,
        in_place: FxHashSet::default(),
        names: FxHashSet::default(),
    };
    walk_parented_exprs(source.ast(), &mut lookups);
    let body = &source.ast().body;
    let bound: FxHashMap<TextSize, TextRange> = filter_map_over_stmts(body, |stmt| {
        let (target, value) = single_name_assignment(stmt)?;
        Some((target.start(), value?.as_dict_expr()?.range()))
    })
    .into_iter()
    .collect();
    let nonlocal: FxHashSet<&str> = filter_map_over_stmts(body, Stmt::as_nonlocal_stmt)
        .into_iter()
        .flat_map(|nonlocal| &nonlocal.names)
        .map(|name| name.as_str())
        .collect();
    let exports = DunderAll::of(source);
    let mut sorted = lookups.in_place;
    sorted.extend(bindings.sole_writes().filter_map(|(write, binding)| {
        let dict = *bound.get(&write)?;
        let name = bindings.binding_name(binding);
        let escapes = nonlocal.contains(name)
            || bindings.is_deleted(name)
            || (bindings.at_module_scope(binding) && exports.exports(name))
            || !bindings
                .read_offsets(binding)
                .iter()
                .all(|read| lookups.names.contains(read));
        (!escapes).then_some(dict)
    }));
    sorted
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{at, parse};

    #[rstest]
    #[case::annotated("D: dict = {'b': 1, 'a': 2}\nx = D['a']\n")]
    #[case::contained("D = {'b': 1, 'a': 2}\nx = 'a' in D\n")]
    #[case::deleted_key("D = {'b': 1, 'a': 2}\ndel D['a']\n")]
    #[case::function_local("def f(k):\n    D = {'b': 1, 'a': 2}\n    return D[k]\n")]
    #[case::get("D = {'b': 1, 'a': 2}\nx = D.get('a')\n")]
    #[case::get_uncalled("D = {'b': 1, 'a': 2}\nx = max(xs, key=D.get)\n")]
    #[case::global_read("D = {'b': 1, 'a': 2}\n\n\ndef f():\n    global D\n    return D['a']\n")]
    #[case::in_place_contained("x = k in {'b': 1, 'a': 2}\n")]
    #[case::in_place_get("x = {'b': 1, 'a': 2}.get(k)\n")]
    #[case::in_place_not_contained("x = k not in {'b': 1, 'a': 2}\n")]
    #[case::in_place_subscript("x = {'b': 1, 'a': 2}[k]\n")]
    #[case::len("D = {'b': 1, 'a': 2}\nx = len(D)\n")]
    #[case::local_named_in_all(
        "__all__ = ['D']\n\n\ndef f(k):\n    D = {'b': 1, 'a': 2}\n    return D[k]\n"
    )]
    #[case::not_contained("D = {'b': 1, 'a': 2}\nx = 'a' not in D\n")]
    #[case::read_from_a_function("D = {'b': 1, 'a': 2}\n\n\ndef f(k):\n    return D[k]\n")]
    #[case::shadowed_by_a_parameter("D = {'b': 1, 'a': 2}\n\n\ndef f(D):\n    return list(D)\n")]
    #[case::sorted("D = {'b': 1, 'a': 2}\nx = sorted(D)\n")]
    #[case::sorted_with_a_key("D = {'b': 1, 'a': 2}\nx = sorted(D, key=D.get)\n")]
    #[case::stored("D = {'b': 1, 'a': 2}\nD['c'] = 3\n")]
    #[case::subscript("D = {'b': 1, 'a': 2}\nx = D['a']\n")]
    #[case::unread("D = {'b': 1, 'a': 2}\n")]
    #[case::unread_public_beside_all("__all__ = ['E']\nD = {'b': 1, 'a': 2}\nE = 1\n")]
    fn lookup_dicts_sorts_a_dict_every_use_looks_up(#[case] src: &str) {
        let source = parse(src);
        let dict = at(source.text(), "{'b': 1, 'a': 2}");
        assert!(lookup_dicts(&source).contains(&dict));
    }

    #[rstest]
    #[case::aliased("D = {'b': 1, 'a': 2}\nE = D\n")]
    #[case::attribute("self.d = {'b': 1, 'a': 2}\n")]
    #[case::augmented("D = {'b': 1, 'a': 2}\nD |= {'c': 3}\n")]
    #[case::chained("D = E = {'b': 1, 'a': 2}\nx = D['a']\n")]
    #[case::class_body("class K:\n    D = {'b': 1, 'a': 2}\n")]
    #[case::compared("D = {'b': 1, 'a': 2}\nx = D == {}\n")]
    #[case::comprehended("D = {'b': 1, 'a': 2}\nx = [k for k in D]\n")]
    #[case::deleted("D = {'b': 1, 'a': 2}\ndel D\n")]
    #[case::double_star("D = {'b': 1, 'a': 2}\nf(**D)\n")]
    #[case::formatted("D = {'b': 1, 'a': 2}\nx = f'{D}'\n")]
    #[case::global_rebound("D = {'b': 1, 'a': 2}\n\n\ndef f():\n    global D\n    D = {}\n")]
    #[case::in_place_call("f({'b': 1, 'a': 2})\n")]
    #[case::in_place_items("x = {'b': 1, 'a': 2}.items()\n")]
    #[case::in_place_left_of_in("x = {'b': 1, 'a': 2} in xs\n")]
    #[case::in_place_len("x = len({'b': 1, 'a': 2})\n")]
    #[case::in_place_sorted("x = sorted({'b': 1, 'a': 2})\n")]
    #[case::items("D = {'b': 1, 'a': 2}\nx = D.items()\n")]
    #[case::iterated("D = {'b': 1, 'a': 2}\nfor k in D:\n    pass\n")]
    #[case::iterator("D = {'b': 1, 'a': 2}\nx = next(iter(D))\n")]
    #[case::json("import json\nD = {'b': 1, 'a': 2}\nx = json.dumps(D)\n")]
    #[case::keys("D = {'b': 1, 'a': 2}\nx = D.keys()\n")]
    #[case::len_shadowed("len = size\nD = {'b': 1, 'a': 2}\nx = len(D)\n")]
    #[case::listed("D = {'b': 1, 'a': 2}\nx = list(D)\n")]
    #[case::listed_in_all("__all__ = ['D']\nD = {'b': 1, 'a': 2}\n")]
    #[case::method_len("D = {'b': 1, 'a': 2}\nx = obj.len(D)\n")]
    #[case::nested("D = {'k': {'b': 1, 'a': 2}}\nx = D['k']\n")]
    #[case::nonlocal_rebound(
        "def f():\n    D = {'b': 1, 'a': 2}\n\n    def g():\n        nonlocal D\n        D = {}\n"
    )]
    #[case::passed("D = {'b': 1, 'a': 2}\nf(D)\n")]
    #[case::popped("D = {'b': 1, 'a': 2}\nx = D.popitem()\n")]
    #[case::rebound("D = {'b': 1, 'a': 2}\nD = {}\n")]
    #[case::returned("def f():\n    D = {'b': 1, 'a': 2}\n    return D\n")]
    #[case::reversed("D = {'b': 1, 'a': 2}\nx = reversed(D)\n")]
    #[case::sorted_beside_another("D = {'b': 1, 'a': 2}\nx = sorted(xs, D)\n")]
    #[case::sorted_items("D = {'b': 1, 'a': 2}\nx = sorted(D.items())\n")]
    #[case::starred("D = {'b': 1, 'a': 2}\nx = [*D]\n")]
    #[case::subscript_key("D = {'b': 1, 'a': 2}\nx = E[D]\n")]
    #[case::tupled("D = {'b': 1, 'a': 2}\nx = tuple(D)\n")]
    #[case::unpacked("D = {'b': 1, 'a': 2}\nx = {**D}\n")]
    #[case::unreadable_all("__all__ = names()\nD = {'b': 1, 'a': 2}\n")]
    #[case::values("D = {'b': 1, 'a': 2}\nx = D.values()\n")]
    #[case::walrus("if (D := {'b': 1, 'a': 2}):\n    x = D['a']\n")]
    #[case::yielded("def f():\n    D = {'b': 1, 'a': 2}\n    yield D\n")]
    fn lookup_dicts_keeps_a_dict_any_other_use_reads(#[case] src: &str) {
        let source = parse(src);
        let dict = at(source.text(), "{'b': 1, 'a': 2}");
        assert!(!lookup_dicts(&source).contains(&dict));
    }
}
