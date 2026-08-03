//! Reads an expression into the rewrite it earns.

use ruff_python_ast::{
    Comprehension, Expr, ExprCall, ExprGenerator, ExprListComp, ExprSetComp, ExprTuple, Keyword,
    comparable::ComparableExpr,
};

use super::constructor::Constructor;
use crate::primitives::binding::sequence_elts;

/// The rewrite an expression earns, each variant naming what stands in
/// place of the source it covers.
pub(super) enum Plan<'a> {
    /// `ctor(iter)`, the form a copying comprehension settles at.
    Collapse { ctor: Constructor, iter: &'a Expr },
    /// Fixed text standing in for the whole expression.
    Fixed(&'static str),
    /// `dict(a=1)` read as the keywords it carries.
    Keywords(&'a [Keyword]),
    /// A dict built from `inner`'s two-element members.
    Pairs {
        inner: &'a Expr,
        pairs: Vec<&'a ExprTuple>,
    },
    /// `inner` rewrapped in `ctor`'s own delimiters.
    Rewrap { ctor: Constructor, inner: &'a Expr },
}

/// The rewrite `expr` earns, or `None` where it stands as written.
/// `rebound` names the constructors the module binds itself, each of
/// which no longer reaches the builtin.
pub(super) fn plan_for<'a>(expr: &'a Expr, rebound: &[Constructor]) -> Option<Plan<'a>> {
    match expr {
        Expr::Call(call) => call_plan(call, rebound),
        Expr::DictComp(comp) => bare_plan(
            Constructor::Dict,
            rebound,
            &comp.generators,
            &[comp.key.as_deref()?, &comp.value],
        ),
        Expr::ListComp(comp) => {
            bare_plan(Constructor::List, rebound, &comp.generators, &[&comp.elt])
        }
        Expr::SetComp(comp) => bare_plan(Constructor::Set, rebound, &comp.generators, &[&comp.elt]),
        _ => None,
    }
}

/// The rewrite a single-argument constructor call earns.
fn argument_plan(ctor: Constructor, arg: &Expr) -> Option<Plan<'_>> {
    match ctor {
        Constructor::Dict => dict_plan(arg),
        Constructor::List => list_plan(arg),
        Constructor::Set => set_plan(arg),
        Constructor::Tuple => tuple_plan(arg),
    }
}

/// The rewrite a bare comprehension earns, held where the module binds
/// the constructor its collapse would name.
fn bare_plan<'a>(
    ctor: Constructor,
    rebound: &[Constructor],
    generators: &'a [Comprehension],
    elements: &[&'a Expr],
) -> Option<Plan<'a>> {
    if rebound.contains(&ctor) {
        return None;
    }
    comprehension_plan(ctor, None, generators, elements)
}

/// The rewrite a call earns, read from its callee and argument list.
fn call_plan<'a>(call: &'a ExprCall, rebound: &[Constructor]) -> Option<Plan<'a>> {
    let ctor = constructor_of(call)?;
    if rebound.contains(&ctor) {
        return None;
    }
    match (ctor, &*call.arguments.args, &*call.arguments.keywords) {
        (Constructor::Dict, [], []) => Some(Plan::Fixed("{}")),
        (Constructor::List, [], []) => Some(Plan::Fixed("[]")),
        (Constructor::Tuple, [], []) => Some(Plan::Fixed("()")),
        (Constructor::Dict, [], keywords) if keywords.iter().all(|kw| kw.arg.is_some()) => {
            Some(Plan::Keywords(keywords))
        }
        (_, [arg], []) => argument_plan(ctor, arg),
        _ => None,
    }
}

/// True when `expr` calls one of the four constructors.
fn calls_constructor(expr: &Expr) -> bool {
    expr.as_call_expr().and_then(constructor_of).is_some()
}

/// The rewrite `ctor(iter)` settles at, folding in one more collapse
/// where `iter` is itself a form the constructor absorbs. Declines where
/// `iter` calls a constructor, leaving the source as written rather than
/// emitting the doubled `ctor(ctor(x))`.
fn collapse_plan(ctor: Constructor, iter: &Expr) -> Option<Plan<'_>> {
    argument_plan(ctor, iter)
        .or_else(|| (!calls_constructor(iter)).then_some(Plan::Collapse { ctor, iter }))
}

/// The rewrite a comprehension over `elements` earns, collapsing where
/// it copies its input and rewrapping `wrapper` where it does not. A
/// bare comprehension carries no wrapper and yields nothing.
fn comprehension_plan<'a>(
    ctor: Constructor,
    wrapper: Option<&'a Expr>,
    generators: &'a [Comprehension],
    elements: &[&'a Expr],
) -> Option<Plan<'a>> {
    if elements.iter().any(|element| element.is_starred_expr()) {
        return None;
    }
    match copy_iter(generators, elements) {
        Some(iter) => collapse_plan(ctor, iter),
        None => wrapper.map(|inner| Plan::Rewrap { ctor, inner }),
    }
}

/// The constructor `call` names, or `None` where it calls anything else.
fn constructor_of(call: &ExprCall) -> Option<Constructor> {
    Constructor::from_name(call.func.as_name_expr()?.id.as_str())
}

/// The iterable a copying comprehension walks, or `None` where the
/// comprehension filters, transforms, chains, or runs async.
fn copy_iter<'a>(generators: &'a [Comprehension], elements: &[&Expr]) -> Option<&'a Expr> {
    let [generator] = generators else {
        return None;
    };
    if generator.is_async || !generator.ifs.is_empty() {
        return None;
    }
    let copies = match elements {
        [element] => same(element, &generator.target),
        _ => sequence_elts(&generator.target).is_some_and(|targets| {
            targets.len() == elements.len()
                && targets
                    .iter()
                    .zip(elements)
                    .all(|(target, element)| same(element, target))
        }),
    };
    copies.then_some(&generator.iter)
}

/// The rewrite `dict`'s sole argument earns.
fn dict_plan(arg: &Expr) -> Option<Plan<'_>> {
    match arg {
        Expr::Dict(_) => Some(Plan::Rewrap {
            ctor: Constructor::Dict,
            inner: arg,
        }),
        Expr::DictComp(comp) => comprehension_plan(
            Constructor::Dict,
            Some(arg),
            &comp.generators,
            &[comp.key.as_deref()?, &comp.value],
        ),
        Expr::Generator(ExprGenerator {
            elt, generators, ..
        })
        | Expr::ListComp(ExprListComp {
            elt, generators, ..
        }) => pair_comprehension_plan(arg, generators, elt),
        _ => {
            let pairs = sequence_elts(arg)?
                .iter()
                .map(pair)
                .collect::<Option<Vec<_>>>()?;
            Some(Plan::Pairs { inner: arg, pairs })
        }
    }
}

/// The rewrite `list`'s sole argument earns.
fn list_plan(arg: &Expr) -> Option<Plan<'_>> {
    match arg {
        Expr::Generator(ExprGenerator {
            elt, generators, ..
        })
        | Expr::ListComp(ExprListComp {
            elt, generators, ..
        }) => comprehension_plan(Constructor::List, Some(arg), generators, &[elt]),
        Expr::List(_) | Expr::Tuple(_) => Some(Plan::Rewrap {
            ctor: Constructor::List,
            inner: arg,
        }),
        _ => None,
    }
}

/// `expr` read as a two-element tuple carrying no starred member, the
/// shape one dict entry is built from.
fn pair(expr: &Expr) -> Option<&ExprTuple> {
    expr.as_tuple_expr()
        .filter(|tuple| tuple.len() == 2 && !tuple.iter().any(Expr::is_starred_expr))
}

/// The rewrite a pair-yielding comprehension earns as `dict`'s argument.
fn pair_comprehension_plan<'a>(
    inner: &'a Expr,
    generators: &'a [Comprehension],
    elt: &'a Expr,
) -> Option<Plan<'a>> {
    let pair = pair(elt)?;
    match copy_iter(generators, &[elt]) {
        Some(iter) => collapse_plan(Constructor::Dict, iter),
        None => Some(Plan::Pairs {
            inner,
            pairs: vec![pair],
        }),
    }
}

/// True when two expressions carry the same shape, the test that reads a
/// comprehension's element against its target.
fn same(a: &Expr, b: &Expr) -> bool {
    ComparableExpr::from(a) == ComparableExpr::from(b)
}

/// The rewrite `set`'s sole argument earns. An empty sequence reaches
/// `set()` rather than a brace form, because `{}` names an empty dict.
fn set_plan(arg: &Expr) -> Option<Plan<'_>> {
    match arg {
        Expr::Generator(ExprGenerator {
            elt, generators, ..
        })
        | Expr::ListComp(ExprListComp {
            elt, generators, ..
        })
        | Expr::SetComp(ExprSetComp {
            elt, generators, ..
        }) => comprehension_plan(Constructor::Set, Some(arg), generators, &[elt]),
        Expr::List(_) | Expr::Set(_) | Expr::Tuple(_) => match sequence_elts(arg) {
            Some([]) => Some(Plan::Fixed("set()")),
            _ => Some(Plan::Rewrap {
                ctor: Constructor::Set,
                inner: arg,
            }),
        },
        _ => None,
    }
}

/// The rewrite `tuple`'s sole argument earns. A comprehension is absent
/// here, because Python spells no tuple comprehension.
fn tuple_plan(arg: &Expr) -> Option<Plan<'_>> {
    matches!(arg, Expr::List(_) | Expr::Tuple(_)).then_some(Plan::Rewrap {
        ctor: Constructor::Tuple,
        inner: arg,
    })
}
