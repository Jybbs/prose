//! Leaf-edit collection for `alphabetize`. A single AST walk emits one
//! non-overlapping edit per outermost reordering structure and maps
//! each function docstring to its signature-order names, the mirror key
//! the docstring-entry sort consumes. Positional-or-keyword parameters
//! never reorder, since no single-file rewrite can keep every caller's
//! positional binding intact. Only the keyword-only block sorts.

use std::{borrow::Cow, collections::HashMap};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Expr, ExprCall, ExprDict, ExprLambda, ExprSet, Identifier, Parameters, Stmt, StmtAssign,
    StmtDelete,
    visitor::{Visitor as AstVisitor, walk_expr, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::dict::rewrite_dict_text;
use crate::{
    primitives::{
        binding::single_name_target,
        docstring::{body_docstring, entry_carrying_sections, rewrite_docstrings},
        edit::{apply_inline_edits, narrowed_replacement},
        orderer::{any_sibling_shares_line, permute_full, reorder_separated, reorder_text},
        params::classify_param,
    },
    source::Source,
};

struct LeafCollector<'a> {
    edits: Vec<Edit>,
    param_docs: HashMap<TextSize, Vec<&'a str>>,
    sort_dunder_lists: bool,
    source: &'a Source,
}

impl<'a> LeafCollector<'a> {
    fn emit_alias_run(&mut self, names: &'a [Alias]) {
        self.try_emit_inline_reorder(names, |a| Some(a.name.as_str()));
    }

    fn emit_call(&mut self, c: &'a ExprCall) {
        for chunk in c.arguments.keywords.split(|kw| kw.arg.is_none()) {
            self.try_emit_inline_reorder(chunk, |kw| kw.arg.as_deref());
        }
    }

    fn emit_delete(&mut self, d: &'a StmtDelete) {
        self.try_emit_inline_reorder(&d.targets, |t| Some(self.source.slice(t)));
    }

    fn emit_dict(&mut self, d: &'a ExprDict) {
        if let Some((span, text)) = rewrite_dict_text(self.source, d, &self.edits) {
            self.fold_into(span, text);
        }
    }

    fn emit_dunder_list(&mut self, assign: &'a StmtAssign) {
        if !self.sort_dunder_lists {
            return;
        }
        let Some(name) = single_name_target(assign) else {
            return;
        };
        if !matches!(name, "__all__" | "__slots__") {
            return;
        }
        let Some(elements) = sequence_elts(&assign.value) else {
            return;
        };
        self.try_emit_inline_reorder(elements, |e| {
            Some(e.as_string_literal_expr()?.value.to_str())
        });
    }

    fn emit_id_run(&mut self, names: &'a [Identifier]) {
        self.try_emit_inline_reorder(names, |id| Some(id.as_str()));
    }

    fn emit_lambda(&mut self, l: &'a ExprLambda) {
        if let Some(params) = l.parameters.as_deref() {
            self.emit_parameters(params);
        }
    }

    /// Sorts only the keyword-only block. A keyword-only parameter
    /// binds by name at every call site, so reordering it preserves
    /// behavior, whereas a positional-or-keyword parameter does not and
    /// holds its source slot.
    fn emit_parameters(&mut self, params: &'a Parameters) {
        self.try_emit_inline_reorder(&params.kwonlyargs, classify_param);
    }

    fn emit_set(&mut self, s: &'a ExprSet) {
        self.try_emit_inline_reorder(&s.elts, |e| {
            (!e.is_starred_expr()).then_some(self.source.slice(e))
        });
    }

    /// Replaces the leaf edits nested inside `span` with a single edit
    /// carrying `folded`, that span reordered with the nested edits
    /// already applied. A `Cow::Borrowed` folded nothing, so emits
    /// nothing. The insert keeps `edits` sorted by start.
    fn fold_into(&mut self, span: TextRange, folded: Cow<'a, str>) {
        let Cow::Owned(text) = folded else {
            return;
        };
        self.edits.retain(|e| !span.contains_range(e.range()));
        insert_by_start(&mut self.edits, Edit::range_replacement(text, span));
    }

    fn try_emit_inline_reorder<T, S>(
        &mut self,
        items: &'a [T],
        classify: impl FnMut(&'a T) -> Option<S>,
    ) where
        T: Ranged,
        S: Ord,
    {
        if items.len() < 2 {
            return;
        }
        let source = self.source;
        let render = |_: usize, block| apply_inline_edits(source, block, &self.edits);
        // One member per line routes through `reorder_separated` so each
        // trailing comment travels with its member. A single-line or
        // atomics-packed group shares lines, so the lighter `reorder_text`
        // keeps its verbatim gaps.
        let one_per_line = !any_sibling_shares_line(source, items);
        let (folded, span) = if one_per_line {
            reorder_separated(source, items, classify, render)
        } else {
            reorder_text(source, items, classify, render)
        };
        self.fold_into(span, folded);
    }
}

impl<'a> AstVisitor<'a> for LeafCollector<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        walk_expr(self, expr);
        match expr {
            Expr::Call(c) => self.emit_call(c),
            Expr::Dict(d) => self.emit_dict(d),
            Expr::Lambda(l) => self.emit_lambda(l),
            Expr::Set(s) => self.emit_set(s),
            _ => {}
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        walk_stmt(self, stmt);
        match stmt {
            Stmt::Assign(a) => self.emit_dunder_list(a),
            Stmt::Delete(d) => self.emit_delete(d),
            Stmt::FunctionDef(f) => {
                if let Some(lit) = body_docstring(&f.body) {
                    self.param_docs
                        .insert(lit.start(), signature_order(&f.parameters));
                }
                self.emit_parameters(&f.parameters);
            }
            Stmt::Global(g) => self.emit_id_run(&g.names),
            Stmt::Import(i) => self.emit_alias_run(&i.names),
            Stmt::ImportFrom(i) => self.emit_alias_run(&i.names),
            Stmt::Nonlocal(n) => self.emit_id_run(&n.names),
            _ => {}
        }
    }
}

/// Walks every docstring in `source` and emits one edit per
/// entry-carrying Google-style section whose `name: description`
/// entries are out of order. An entry naming a parameter of the
/// documented signature takes that parameter's position as the rule
/// leaves the signature, and every other entry sinks below them,
/// alphabetized by name. Module and class docstrings carry no
/// signature, so their sections alphabetize throughout. Each edit
/// replaces the section's entries-span with the reordered text.
/// Returns an empty list when no docstring carries a sortable section.
pub(super) fn collect_docstring_entry_edits(
    source: &Source,
    param_docs: &HashMap<TextSize, Vec<&str>>,
) -> Vec<Edit> {
    rewrite_docstrings(source, |source, lit, edits| {
        let signature = param_docs.get(&lit.start());
        for entries in entry_carrying_sections(source, lit) {
            let (cow, span) = reorder_text(
                source,
                &entries,
                |entry| Some(entry_key(entry.name, signature)),
                |_, block| Cow::Borrowed(source.slice(block)),
            );
            let Cow::Owned(text) = cow else {
                continue;
            };
            edits.extend(narrowed_replacement(source, span, text));
        }
    })
}

/// Walks the AST collecting one non-overlapping leaf edit per outermost
/// reordering structure, each folding its nested reorders in, and maps
/// each function docstring's start to its signature-order names, the
/// mirror key for docstring-entry sorting.
pub(super) fn collect_leaf_edits(
    source: &Source,
    sort_dunder_lists: bool,
) -> (Vec<Edit>, HashMap<TextSize, Vec<&str>>) {
    let mut collector = LeafCollector {
        edits: Vec::new(),
        param_docs: HashMap::new(),
        sort_dunder_lists,
        source,
    };
    collector.visit_body(&source.ast().body);
    (collector.edits, collector.param_docs)
}

/// Composite docstring-entry sort key. An entry naming a signature
/// parameter takes that parameter's position, and any other entry
/// sinks below the signature's, alphabetized by name.
fn entry_key<'e>(name: &'e str, signature: Option<&Vec<&str>>) -> (usize, &'e str) {
    match signature.and_then(|names| names.iter().position(|&n| n == name)) {
        Some(i) => (i, ""),
        None => (usize::MAX, name),
    }
}

/// Inserts `edit` into a `Vec<Edit>` kept sorted by `range().start()`,
/// preserving that order.
fn insert_by_start(edits: &mut Vec<Edit>, edit: Edit) {
    let slot = edits.partition_point(|e| e.range().start() < edit.range().start());
    edits.insert(slot, edit);
}

/// Returns the elements of a list or tuple expression. `None` for
/// any other shape.
fn sequence_elts(expr: &Expr) -> Option<&[Expr]> {
    match expr {
        Expr::List(l) => Some(&l.elts),
        Expr::Tuple(t) => Some(&t.elts),
        _ => None,
    }
}

/// Returns the parameter names in the order the rule leaves the
/// signature: positional-only and positional-or-keyword in source
/// order, then `*args`, then the keyword-only block sorted, then
/// `**kwargs`.
fn signature_order(params: &Parameters) -> Vec<&str> {
    let mut names: Vec<&str> = params
        .posonlyargs
        .iter()
        .chain(&params.args)
        .map(|p| p.name().as_str())
        .collect();
    names.extend(params.vararg.as_deref().map(|p| p.name.as_str()));
    let mut order: Vec<usize> = (0..params.kwonlyargs.len()).collect();
    permute_full(&mut order, &params.kwonlyargs, classify_param);
    names.extend(order.iter().map(|&i| params.kwonlyargs[i].name().as_str()));
    names.extend(params.kwarg.as_deref().map(|p| p.name.as_str()));
    names
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, parse};

    #[rstest]
    #[case(indoc! {"
        class C:
            def m(self, b, a):
                \"\"\"Summary.

                Args:
                    b: two
                    a: one

                Raises:
                    ValueError: bad
                    KeyError: missing
                \"\"\"
    "})]
    #[case(indoc! {"
        def f(b, a):
            \"\"\"Summary.

            Args:
                b: two
                a: one

            Raises:
                ValueError: bad
                KeyError: missing
            \"\"\"
    "})]
    fn collect_docstring_entry_edits_mirrors_source_order_signature(#[case] src: &str) {
        let source = parse(src);
        let (_, param_docs) = collect_leaf_edits(&source, true);
        let edits = collect_docstring_entry_edits(&source, &param_docs);
        let text = applied_text(&source, edits);
        let pos = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} present"))
        };
        assert!(
            pos("b: two") < pos("a: one"),
            "parameter entries mirror the un-reordered signature"
        );
        assert!(
            pos("KeyError: missing") < pos("ValueError: bad"),
            "non-parameter entries still sort"
        );
    }

    #[test]
    fn collect_docstring_entry_edits_mirrors_vararg_and_kwarg_positions() {
        let src = indoc! {"
            def f(beta, alpha, *zebra, **apple):
                \"\"\"Summary.

                Args:
                    apple: d
                    zebra: c
                    beta: a
                    alpha: b
                \"\"\"
        "};
        let source = parse(src);
        let (_, param_docs) = collect_leaf_edits(&source, true);
        let edits = collect_docstring_entry_edits(&source, &param_docs);
        let text = applied_text(&source, edits);
        let pos = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} present"))
        };
        assert!(
            pos("zebra:") < pos("apple:"),
            "the vararg mirrors ahead of the kwarg, both in signature order"
        );
    }

    #[test]
    fn collect_docstring_entry_edits_sinks_stale_entries_below_params() {
        let src = indoc! {"
            class Catalog:
                def update(self, target, source):
                    \"\"\"Apply ``source`` onto ``target``.

                    Args:
                        source: Mapping providing new values.
                        retries: Attempts before giving up.
                        target: Mapping receiving the update.
                    \"\"\"
        "};
        let source = parse(src);
        let (_, param_docs) = collect_leaf_edits(&source, true);
        let edits = collect_docstring_entry_edits(&source, &param_docs);
        let text = applied_text(&source, edits);
        let pos = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} present"))
        };
        assert!(
            pos("target:") < pos("source:") && pos("source:") < pos("retries:"),
            "parameter entries mirror the signature and the stale entry sinks"
        );
    }

    #[rstest]
    #[case("def m(b, a): pass\n", "def m(b, a): pass\n")]
    #[case(
        "class C:\n    def m(self, b, a): pass\n",
        "class C:\n    def m(self, b, a): pass\n"
    )]
    #[case(
        "def m(self, b, a, *, d=1, c=2): pass\n",
        "def m(self, b, a, *, c=2, d=1): pass\n"
    )]
    #[case(
        "class C:\n    def m(self, b, a, *, d=1, c=2): pass\n",
        "class C:\n    def m(self, b, a, *, c=2, d=1): pass\n"
    )]
    #[case("key = lambda b, a: 0\n", "key = lambda b, a: 0\n")]
    #[case("key = lambda b, a, *, d, c: 0\n", "key = lambda b, a, *, c, d: 0\n")]
    #[case(
        "def m(b, a):\n    foo(b=2, a=1)\n",
        "def m(b, a):\n    foo(a=1, b=2)\n"
    )]
    fn collect_leaf_edits_holds_positionals_and_sorts_keyword_only(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let (edits, _) = collect_leaf_edits(&source, true);
        assert_eq!(applied_text(&source, edits), expected);
    }

    #[test]
    fn collect_leaf_edits_yields_edits_in_source_order() {
        let src = indoc! {"
            import b, a
            from m import d, c
            __all__ = ['z', 'y']
            x = {z, y}
            foo(b=2, a=1)
        "};
        let source = parse(src);
        let (edits, _) = collect_leaf_edits(&source, true);
        assert!(edits.len() >= 5, "fixture must trigger multiple producers");
        assert!(
            edits.is_sorted(),
            "leaf edits must be emitted in source order, since partition_point in apply_inline_edits relies on it",
        );
    }
}
