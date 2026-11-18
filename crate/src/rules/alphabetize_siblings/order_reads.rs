//! The values whose iteration order the module reads. A form reads the
//! order of the value it takes: a `for` loop or a comprehension over it,
//! `.keys()`, `.values()`, or `.items()` on it, `iter`, `list`, `tuple`,
//! `reversed`, or `enumerate` over it, and `*` or `**` unpacking it.
//! `json.dumps` and `json.dump` read the order of every dict literal
//! nested in the value they take as well. The value is the expression
//! the form takes, or, where that expression is a name, each value a
//! `name = value`, `name: T = value`, `name := value`, or augmented
//! `name |= value` write binds to the binding that read resolves to,
//! and both branches of a conditional value. A name reached through an
//! alias, an attribute, a subscript, or another function's parameter is
//! not followed.

use ruff_python_ast::{
    Comprehension, Expr, Stmt,
    name::UnqualifiedName,
    visitor::{Visitor, walk_comprehension, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    primitives::{binding::BindingAnalysis, walk::walk_stmt},
    source::Source,
};

/// The subjects of every order-reading form a walk reaches, a name read
/// as its offset and any other expression as itself, with the JSON
/// subjects apart in `deep` and `deep_names`, and the value each name
/// write binds keyed by the write's offset.
#[derive(Default)]
struct OrderReads<'a> {
    bound: FxHashMap<TextSize, &'a Expr>,
    deep: Vec<&'a Expr>,
    deep_names: FxHashSet<TextSize>,
    names: FxHashSet<TextSize>,
    values: Vec<&'a Expr>,
}

impl<'a> OrderReads<'a> {
    fn bind(&mut self, target: &Expr, value: &'a Expr) {
        if let Expr::Name(name) = target {
            self.bound.insert(name.start(), value);
        }
    }

    fn read(&mut self, subject: &'a Expr) {
        if let Expr::Name(name) = subject {
            self.names.insert(name.start());
        } else {
            self.values.push(subject);
        }
    }

    fn read_deep(&mut self, subject: &'a Expr) {
        if let Expr::Name(name) = subject {
            self.deep_names.insert(name.start());
        } else {
            self.deep.push(subject);
        }
    }

    /// The subjects read directly, then the values bound to each binding
    /// read at one of `names`.
    fn resolved<'b>(
        &'b self,
        bindings: &'b BindingAnalysis,
        subjects: &'b [&'a Expr],
        names: &'b FxHashSet<TextSize>,
    ) -> impl Iterator<Item = &'a Expr> + use<'a, 'b> {
        subjects.iter().copied().chain(
            bindings
                .writes_read_at(names)
                .filter_map(|offset| self.bound.get(&offset).copied()),
        )
    }
}

impl<'a> Visitor<'a> for OrderReads<'a> {
    fn visit_comprehension(&mut self, comprehension: &'a Comprehension) {
        self.read(&comprehension.iter);
        walk_comprehension(self, comprehension);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(attribute)
                if matches!(attribute.attr.as_str(), "items" | "keys" | "values") =>
            {
                self.read(&attribute.value);
            }
            Expr::Call(call) => {
                if let [first, ..] = &*call.arguments.args {
                    match OrderRead::of(&call.func) {
                        Some(OrderRead::Nested) => self.read_deep(first),
                        Some(OrderRead::Shallow) => self.read(first),
                        None => {}
                    }
                }
                for keyword in &call.arguments.keywords {
                    if keyword.arg.is_none() {
                        self.read(&keyword.value);
                    }
                }
            }
            Expr::Dict(dict) => {
                for item in &dict.items {
                    if item.key.is_none() {
                        self.read(&item.value);
                    }
                }
            }
            Expr::Named(named) => self.bind(&named.target, &named.value),
            Expr::Starred(starred) => self.read(&starred.value),
            _ => {}
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::AnnAssign(assign) => {
                if let Some(value) = &assign.value {
                    self.bind(&assign.target, value);
                }
            }
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.bind(target, &assign.value);
                }
            }
            Stmt::AugAssign(assign) => self.bind(&assign.target, &assign.value),
            Stmt::For(for_stmt) => self.read(&for_stmt.iter),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Every dict literal a walk reaches, added to `held`.
struct NestedDicts<'h> {
    held: &'h mut FxHashSet<TextRange>,
}

impl<'a> Visitor<'a> for NestedDicts<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.is_dict_expr() {
            self.held.insert(expr.range());
        }
        walk_expr(self, expr);
    }
}

/// How far a call reads the order of its first argument.
enum OrderRead {
    Nested,
    Shallow,
}

impl OrderRead {
    /// The order `func` reads in its first argument: the argument's own
    /// for `enumerate`, `iter`, `list`, `reversed`, and `tuple`, and that
    /// of every dict nested in it for `json.dump` and `json.dumps`.
    fn of(func: &Expr) -> Option<Self> {
        let name = UnqualifiedName::from_expr(func)?;
        match name.segments() {
            ["enumerate" | "iter" | "list" | "reversed" | "tuple"] => Some(Self::Shallow),
            ["json", "dump" | "dumps"] => Some(Self::Nested),
            _ => None,
        }
    }
}

/// Adds the range of `value` to `held`, or, for a conditional value or
/// a walrus, the range of each value it yields.
fn hold(value: &Expr, held: &mut FxHashSet<TextRange>) {
    match value {
        Expr::If(conditional) => {
            hold(&conditional.body, held);
            hold(&conditional.orelse, held);
        }
        Expr::Named(named) => hold(&named.value, held),
        _ => {
            held.insert(value.range());
        }
    }
}

/// The ranges of the values whose iteration order `source` reads, per
/// the module doc.
pub(crate) fn order_read_values(source: &Source) -> FxHashSet<TextRange> {
    let mut reads = OrderReads::default();
    reads.visit_body(&source.ast().body);
    let bindings = source.binding_analysis();
    let mut held = FxHashSet::default();
    for value in reads.resolved(bindings, &reads.values, &reads.names) {
        hold(value, &mut held);
    }
    let mut nested = NestedDicts { held: &mut held };
    for value in reads.resolved(bindings, &reads.deep, &reads.deep_names) {
        nested.visit_expr(value);
    }
    held
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{at, parse};

    #[rstest]
    #[case::annotated("D: dict = {'b': 1, 'a': 2}\nfor k in D:\n    pass\n")]
    #[case::augmented("D = {}\nD |= {'b': 1, 'a': 2}\nprint(list(D))\n")]
    #[case::conditional_body("D = {'b': 1, 'a': 2} if x else {}\nprint(list(D))\n")]
    #[case::conditional_orelse("D = {} if x else {'b': 1, 'a': 2}\nprint(list(D))\n")]
    #[case::comprehension("D = {'b': 1, 'a': 2}\nx = [k for k in D]\n")]
    #[case::direct_items("x = {'b': 1, 'a': 2}.items()\n")]
    #[case::double_star("D = {'b': 1, 'a': 2}\nx = {**D}\n")]
    #[case::double_star_call("D = {'b': 1, 'a': 2}\nf(**D)\n")]
    #[case::enumerate("D = {'b': 1, 'a': 2}\nx = enumerate(D)\n")]
    #[case::for_loop("D = {'b': 1, 'a': 2}\nfor k in D:\n    pass\n")]
    #[case::function_local("def f():\n    D = {'b': 1, 'a': 2}\n    return list(D)\n")]
    #[case::json_dump("import json\nD = {'b': 1, 'a': 2}\njson.dump(D, out)\n")]
    #[case::json_dumps("import json\nD = {'b': 1, 'a': 2}\njson.dumps(D)\n")]
    #[case::json_list_of_dicts("import json\njson.dumps([{'b': 1, 'a': 2}])\n")]
    #[case::json_nested("import json\nD = {'deps': {'b': 1, 'a': 2}}\njson.dumps(D)\n")]
    #[case::json_nested_direct("import json\njson.dump({'deps': {'b': 1, 'a': 2}}, out)\n")]
    #[case::keys("D = {'b': 1, 'a': 2}\nD.keys()\n")]
    #[case::next_iter("D = {'b': 1, 'a': 2}\nnext(iter(D))\n")]
    #[case::read_in_a_function(
        "D = {'b': 1, 'a': 2}\n\n\ndef f():\n    return next(k for k, v in D.items())\n"
    )]
    #[case::reversed("D = {'b': 1, 'a': 2}\nx = reversed(D)\n")]
    #[case::starred("D = {'b': 1, 'a': 2}\nx = [*D]\n")]
    #[case::tuple("D = {'b': 1, 'a': 2}\nx = tuple(D)\n")]
    #[case::values("D = {'b': 1, 'a': 2}\nx = D.values()\n")]
    #[case::walrus("if (D := {'b': 1, 'a': 2}):\n    print(list(D))\n")]
    fn order_read_values_holds_a_dict_an_order_reading_form_takes(#[case] src: &str) {
        let source = parse(src);
        let dict = at(source.text(), "{'b': 1, 'a': 2}");
        assert!(order_read_values(&source).contains(&dict));
    }

    #[rstest]
    #[case::alias("D = {'b': 1, 'a': 2}\nE = D\nlist(E)\n")]
    #[case::get("D = {'b': 1, 'a': 2}\nx = D.get('a')\n")]
    #[case::list_of_dicts("x = list([{'b': 1, 'a': 2}])\n")]
    #[case::lookup("D = {'b': 1, 'a': 2}\nx = D['a']\n")]
    #[case::passed("D = {'b': 1, 'a': 2}\nf(D)\n")]
    #[case::shadowed("D = {'b': 1, 'a': 2}\n\n\ndef f(D):\n    return list(D)\n")]
    #[case::sorted("D = {'b': 1, 'a': 2}\nx = sorted(D)\n")]
    #[case::subscript("C = {'d': {'b': 1, 'a': 2}}\nlist(C['d'])\n")]
    fn order_read_values_skips_a_dict_no_form_reaches(#[case] src: &str) {
        let source = parse(src);
        let dict = at(source.text(), "{'b': 1, 'a': 2}");
        assert!(!order_read_values(&source).contains(&dict));
    }
}
