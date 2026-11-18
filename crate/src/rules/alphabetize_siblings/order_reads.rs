//! The values whose iteration order the module reads. A form reads the
//! order of the value it takes: a `for` loop or a comprehension over it,
//! `.keys()`, `.values()`, or `.items()` on it, `iter`, `list`, `tuple`,
//! `reversed`, `enumerate`, `json.dumps`, or `json.dump` over it, and
//! `*` or `**` unpacking it. The value is the expression the form takes,
//! or, where that expression is a name, each value a direct
//! `name = value` write binds to the binding that read resolves to. A
//! name reached through an alias, an attribute, a subscript, or another
//! function's parameter is not followed.

use ruff_python_ast::{
    Comprehension, Expr, Stmt,
    name::UnqualifiedName,
    visitor::{Visitor, walk_comprehension, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::{primitives::walk::walk_stmt, source::Source};

/// The subjects of every order-reading form a walk reaches, a name read
/// as its offset and any other expression as its range.
#[derive(Default)]
struct OrderReads {
    names: FxHashSet<TextSize>,
    values: FxHashSet<TextRange>,
}

impl OrderReads {
    fn read(&mut self, subject: &Expr) {
        if let Expr::Name(name) = subject {
            self.names.insert(name.start());
        } else {
            self.values.insert(subject.range());
        }
    }
}

impl<'a> Visitor<'a> for OrderReads {
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
                if let [first, ..] = &*call.arguments.args
                    && iterates_its_argument(&call.func)
                {
                    self.read(first);
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
            Expr::Starred(starred) => self.read(&starred.value),
            _ => {}
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::For(for_stmt) = stmt {
            self.read(&for_stmt.iter);
        }
        walk_stmt(self, stmt);
    }
}

/// The ranges of the values whose iteration order `source` reads, per
/// the module doc.
pub(crate) fn order_read_values(source: &Source) -> FxHashSet<TextRange> {
    let mut reads = OrderReads::default();
    reads.visit_body(&source.ast().body);
    let OrderReads { names, mut values } = reads;
    values.extend(source.binding_analysis().values_read_at(&names));
    values
}

/// True when `func` names a callable that walks its first argument in
/// iteration order.
fn iterates_its_argument(func: &Expr) -> bool {
    UnqualifiedName::from_expr(func).is_some_and(|name| {
        matches!(
            name.segments(),
            ["enumerate" | "iter" | "list" | "reversed" | "tuple"] | ["json", "dump" | "dumps"]
        )
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{at, parse};

    #[rstest]
    #[case::annotated("D: dict = {'b': 1, 'a': 2}\nfor k in D:\n    pass\n")]
    #[case::comprehension("D = {'b': 1, 'a': 2}\nx = [k for k in D]\n")]
    #[case::direct_items("x = {'b': 1, 'a': 2}.items()\n")]
    #[case::double_star("D = {'b': 1, 'a': 2}\nx = {**D}\n")]
    #[case::double_star_call("D = {'b': 1, 'a': 2}\nf(**D)\n")]
    #[case::enumerate("D = {'b': 1, 'a': 2}\nx = enumerate(D)\n")]
    #[case::for_loop("D = {'b': 1, 'a': 2}\nfor k in D:\n    pass\n")]
    #[case::function_local("def f():\n    D = {'b': 1, 'a': 2}\n    return list(D)\n")]
    #[case::json_dump("import json\nD = {'b': 1, 'a': 2}\njson.dump(D, out)\n")]
    #[case::json_dumps("import json\nD = {'b': 1, 'a': 2}\njson.dumps(D)\n")]
    #[case::keys("D = {'b': 1, 'a': 2}\nD.keys()\n")]
    #[case::next_iter("D = {'b': 1, 'a': 2}\nnext(iter(D))\n")]
    #[case::read_in_a_function(
        "D = {'b': 1, 'a': 2}\n\n\ndef f():\n    return next(k for k, v in D.items())\n"
    )]
    #[case::reversed("D = {'b': 1, 'a': 2}\nx = reversed(D)\n")]
    #[case::starred("D = {'b': 1, 'a': 2}\nx = [*D]\n")]
    #[case::tuple("D = {'b': 1, 'a': 2}\nx = tuple(D)\n")]
    #[case::values("D = {'b': 1, 'a': 2}\nx = D.values()\n")]
    fn order_read_values_holds_a_dict_an_order_reading_form_takes(#[case] src: &str) {
        let source = parse(src);
        let dict = at(source.text(), "{'b': 1, 'a': 2}");
        assert!(order_read_values(&source).contains(&dict));
    }

    #[rstest]
    #[case::alias("D = {'b': 1, 'a': 2}\nE = D\nlist(E)\n")]
    #[case::get("D = {'b': 1, 'a': 2}\nx = D.get('a')\n")]
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
