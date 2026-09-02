//! Leaf-edit collection for `alphabetize-siblings`. A single AST walk
//! emits one non-overlapping edit per outermost reordering structure,
//! and the docstring-entry sort reads each function's signature-order
//! names as its mirror key. Positional-or-keyword parameters never
//! reorder, since no single-file rewrite can keep every caller's
//! positional binding intact. Only the keyword-only block sorts. A call
//! keyword bound to an effectful value holds its slot while the inert
//! keywords around it sort.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Expr, ExprCall, ExprDict, ExprLambda, ExprSet, Identifier, Parameters, Stmt, StmtAssign,
    StmtDelete,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use super::{dict::rewrite_dict_text, dunder_list, joined_text};
use crate::{
    primitives::{
        binding::sequence_elts,
        comments::has_keep_marker,
        edit::{apply_inline_edits, insert_edit},
        effect::value_is_effectful,
        orderer::{
            permute_full, reorder_separated, reorder_text, reordered_lines_fit,
            swap_relocates_spanning, swap_span_holds, swaps_in_place,
        },
        params::classify_param,
        walk::walk_stmt,
    },
    source::Source,
};

struct LeafCollector<'a> {
    code_width: usize,
    edits: Vec<Edit>,
    sort_dict_keys: bool,
    sort_dunder_lists: bool,
    source: &'a Source,
}

impl<'a> LeafCollector<'a> {
    fn emit_alias_run(&mut self, names: &'a [Alias]) {
        self.try_emit_inline_reorder(names, |a| Some(a.name.as_str()));
    }

    /// Sorts only the keywords binding an inert value.
    fn emit_call(&mut self, c: &'a ExprCall) {
        for chunk in c.arguments.keywords.split(|kw| kw.arg.is_none()) {
            self.try_emit_inline_reorder(chunk, |kw| {
                kw.arg.as_deref().filter(|_| !value_is_effectful(&kw.value))
            });
        }
    }

    /// Sorts only the bare names, a subscript or attribute target
    /// pinning where unbinding it out of order raises.
    fn emit_delete(&mut self, d: &'a StmtDelete) {
        self.try_emit_inline_reorder(&d.targets, |t| {
            t.as_name_expr().map(|name| name.id.as_str())
        });
    }

    fn emit_dict(&mut self, d: &'a ExprDict) {
        if self.sort_dict_keys
            && let Some((span, text)) =
                rewrite_dict_text(self.source, d, &self.edits, self.code_width)
        {
            self.fold_into(span, text);
        }
    }

    fn emit_dunder_list(&mut self, assign: &'a StmtAssign) {
        if self.sort_dunder_lists
            && dunder_list(assign)
            && !has_keep_marker(self.source, &*assign.value)
            && let Some(elements) = sequence_elts(&assign.value)
        {
            self.try_emit_inline_reorder(elements, |e| {
                Some(e.as_string_literal_expr()?.value.to_str())
            });
        }
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

    /// Sorts the set's elements, each keyed on its text as the edits
    /// already collected inside it rewrite it, so an element whose own
    /// entries sort lands where its sorted form will.
    fn emit_set(&mut self, s: &'a ExprSet) {
        let keys: FxHashMap<TextSize, String> = s
            .elts
            .iter()
            .filter(|e| !e.is_starred_expr())
            .map(|e| {
                let placed = apply_inline_edits(self.source, e.range(), &self.edits);
                (e.start(), joined_text(&placed).into_owned())
            })
            .collect();
        self.try_emit_inline_reorder(&s.elts, |e| keys.get(&e.start()).cloned());
    }

    /// Replaces the leaf edits nested inside `span` with a single edit
    /// carrying `text`, that span reordered with the nested edits
    /// already applied. The insert keeps `edits` sorted by start.
    fn fold_into(&mut self, span: TextRange, text: String) {
        self.edits.retain(|e| !span.contains_range(e.range()));
        insert_edit(&mut self.edits, Edit::range_replacement(text, span));
    }

    fn try_emit_inline_reorder<T, S>(
        &mut self,
        items: &'a [T],
        mut classify: impl FnMut(&'a T) -> Option<S>,
    ) where
        T: Ranged,
        S: Ord,
    {
        if items.len() < 2 {
            return;
        }
        let source = self.source;
        // A group sharing lines, opening mid-row, or carrying code in
        // its gaps swaps member slices through `reorder_text`, keeping
        // every gap verbatim, and holds its order where a comment sits
        // in a line-spanning swap span. A one-member-per-line group
        // routes through `reorder_separated` so each trailing comment
        // travels with its member, and a swap widening past the budget
        // and the widest source row holds the group.
        let swapped = swaps_in_place(source, items);
        if swap_span_holds(source, items, swapped) {
            return;
        }
        let render = |_: usize, block| apply_inline_edits(source, block, &self.edits);
        if swapped {
            let mut order: Vec<usize> = (0..items.len()).collect();
            permute_full(&mut order, items, &mut classify);
            if swap_relocates_spanning(source, &order, |idx| items[idx].range()) {
                return;
            }
        }
        let (folded, span) = if swapped {
            reorder_text(source, items, classify, render)
        } else {
            reorder_separated(source, items, classify, render)
        };
        if let Cow::Owned(text) = folded
            && (!swapped || reordered_lines_fit(source, span, &text, self.code_width))
        {
            self.fold_into(span, text);
        }
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
            Stmt::FunctionDef(f) => self.emit_parameters(&f.parameters),
            Stmt::Global(g) => self.emit_id_run(&g.names),
            Stmt::Import(i) => self.emit_alias_run(&i.names),
            Stmt::ImportFrom(i) => self.emit_alias_run(&i.names),
            Stmt::Nonlocal(n) => self.emit_id_run(&n.names),
            _ => {}
        }
    }
}

/// Walks the AST collecting one non-overlapping leaf edit per outermost
/// reordering structure, each folding its nested reorders in.
/// `sort_dict_keys` and `sort_dunder_lists` gate the dict-literal and
/// `__all__` / `__slots__` reorders, every other shape sorting
/// regardless.
pub(super) fn collect_leaf_edits(
    source: &Source,
    code_width: usize,
    sort_dict_keys: bool,
    sort_dunder_lists: bool,
) -> Vec<Edit> {
    let mut collector = LeafCollector {
        code_width,
        edits: Vec::new(),
        sort_dict_keys,
        sort_dunder_lists,
        source,
    };
    collector.visit_body(&source.ast().body);
    collector.edits
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, parse};

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
        let edits = collect_leaf_edits(&source, 88, true, true);
        assert_eq!(applied_text(&source, edits), expected);
    }

    #[rstest]
    fn collect_leaf_edits_skips_a_dunder_list_bound_to_a_non_sequence(
        #[values("__all__ = get_names()\n", "__slots__ = BASE_SLOTS\n")] src: &str,
    ) {
        let source = parse(src);
        let edits = collect_leaf_edits(&source, 88, true, true);
        assert!(edits.is_empty());
    }

    #[rstest]
    #[case("foo(b=make(), a=1)\n", "foo(b=make(), a=1)\n")]
    #[case("foo(z=1, b=make(), a=3)\n", "foo(a=3, b=make(), z=1)\n")]
    #[case("foo(b=obj.attr, a=other.attr)\n", "foo(a=other.attr, b=obj.attr)\n")]
    fn collect_leaf_edits_sorts_only_inert_keyword_values(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let edits = collect_leaf_edits(&source, 88, true, true);
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
        let edits = collect_leaf_edits(&source, 88, true, true);
        assert!(edits.len() >= 5, "fixture must trigger multiple producers");
        assert!(
            edits.is_sorted(),
            "leaf edits must be emitted in source order, since partition_point in \
             apply_inline_edits relies on it",
        );
    }
}
