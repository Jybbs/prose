//! Leaf-edit collection for `alphabetize`. A single AST walk emits one
//! non-overlapping edit per outermost reordering structure, folds in
//! every call-site keyword rewrite that does not overlap one, and maps
//! each function docstring to its signature-order names, the mirror
//! key the docstring-entry sort consumes.

use std::{borrow::Cow, cmp::Reverse, collections::HashMap};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Expr, ExprCall, ExprDict, ExprLambda, ExprSet, Identifier, ParameterWithDefault,
    Parameters, Stmt, StmtAssign, StmtDelete,
    visitor::{Visitor as AstVisitor, walk_expr, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::dict::rewrite_dict_text;
use crate::{
    primitives::{
        call_keywords::{callee_params, keyword_args, module_call_params, pins_positional_params},
        docstring::{body_docstring, entry_carrying_sections, rewrite_docstrings},
        edit::{apply_inline_edits, narrowed_replacement},
        orderer::{assemble_blocks, blocks_span, permute_full, reorder_text},
        scope::{BodyScope, scoped_body},
    },
    source::Source,
};

struct LeafCollector<'a> {
    edits: Vec<Edit>,
    param_docs: HashMap<TextSize, Vec<&'a str>>,
    rewrite_edits: Vec<Edit>,
    rewrite_targets: &'a HashMap<TextSize, &'a Parameters>,
    scope: BodyScope,
    source: &'a Source,
}

impl<'a> LeafCollector<'a> {
    fn emit_alias_run(&mut self, names: &'a [Alias]) {
        self.try_emit_inline_reorder(names, |a| Some(a.name.as_str()));
    }

    fn emit_call(&mut self, c: &'a ExprCall) {
        if self.try_emit_keyword_rewrite(c) {
            return;
        }
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
        let [Expr::Name(target)] = assign.targets.as_slice() else {
            return;
        };
        if !matches!(target.id.as_str(), "__all__" | "__slots__") {
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
            // A class-body lambda is a method in everything but spelling,
            // so its positional-or-keyword order pins like a `def`'s.
            self.emit_parameters(params, self.scope == BodyScope::Class);
        }
    }

    fn emit_parameters(&mut self, params: &'a Parameters, pin_positional: bool) {
        // Positional-only params stay put, because no call-site keyword
        // form can rebind the arguments a reorder would move.
        if !pin_positional {
            self.try_emit_inline_reorder(&params.args, classify_param);
        }
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
        let [first, .., last] = items else {
            return;
        };
        let span = first.range().cover(last.range());
        let folded = reorder_text(self.source, items, classify, |i, _| {
            apply_inline_edits(self.source, items[i].range(), &self.edits)
        });
        self.fold_into(span, folded);
    }

    /// Rewrites a call to a reordered module function, converting each
    /// keyword-eligible positional argument to `name=value` and emitting
    /// the keyword run alphabetized. Returns `false` when the call cannot
    /// take that form, leaving the caller to fall back on the keyword reorder.
    fn try_emit_keyword_rewrite(&mut self, c: &'a ExprCall) -> bool {
        let Some(params) = callee_params(self.rewrite_targets, c) else {
            return false;
        };
        let Some(keywords) = keyword_args(self.source, c, Some(params)) else {
            return false;
        };
        // A call whose positional arguments are all positional-only has
        // nothing to convert, leaving the plain keyword reorder to sort it instead.
        if c.arguments.args.len() <= keywords.posonly_prefix {
            return false;
        }
        let (blocks, keys, rendered): (Vec<TextRange>, Vec<&str>, Vec<Cow<'a, str>>) = keywords
            .args
            .into_iter()
            .map(|arg| (arg.block, arg.name, arg.rendered))
            .multiunzip();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        order.sort_unstable_by_key(|&i| keys[i]);
        let assembled = assemble_blocks(self.source, &blocks, &rendered, &order, |_| None);
        self.rewrite_edits
            .push(Edit::range_replacement(assembled, blocks_span(&blocks)));
        true
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
        let enclosing = self.scope;
        // A compound statement's arms inherit the enclosing scope,
        // matching the `rewrite_stmt` recursion.
        self.scope = scoped_body(stmt).map_or(enclosing, |(_, scope)| scope);
        walk_stmt(self, stmt);
        self.scope = enclosing;
        match stmt {
            Stmt::Assign(a) => self.emit_dunder_list(a),
            Stmt::Delete(d) => self.emit_delete(d),
            // A class-body function's callers routinely live outside the
            // module, where no call-site rewrite can preserve their
            // positional bindings, so its positional-or-keyword order pins.
            Stmt::FunctionDef(f) => {
                let pinned = self.scope == BodyScope::Class || pins_positional_params(f);
                if let Some(lit) = body_docstring(&f.body) {
                    self.param_docs
                        .insert(lit.start(), signature_order(&f.parameters, pinned));
                }
                self.emit_parameters(&f.parameters, pinned);
            }
            Stmt::Global(g) => self.emit_id_run(&g.names),
            Stmt::Import(i) => self.emit_alias_run(&i.names),
            Stmt::ImportFrom(i) => self.emit_alias_run(&i.names),
            Stmt::Nonlocal(n) => self.emit_id_run(&n.names),
            _ => {}
        }
    }
}

/// Maps each in-module call's callee offset to the parameters of the
/// top-level function it resolves to, restricted to functions whose
/// positional `args` reorder under alphabetization.
pub(super) fn call_rewrite_targets(source: &Source) -> HashMap<TextSize, &Parameters> {
    module_call_params(source, |func| args_reorder(&func.parameters))
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
            let cow = reorder_text(
                source,
                &entries,
                |entry| Some(entry_key(entry.name, signature)),
                |_, slice| Cow::Borrowed(slice),
            );
            let Cow::Owned(text) = cow else {
                continue;
            };
            let [first, .., last] = entries.as_slice() else {
                unreachable!("Cow::Owned implies entries.len() >= 2");
            };
            edits.extend(narrowed_replacement(
                source,
                first.range.cover(last.range),
                text,
            ));
        }
    })
}

/// Walks the AST collecting one non-overlapping leaf edit per outermost
/// reordering structure, each folding its nested reorders in, then folds
/// in every call-site keyword rewrite that does not overlap one. Also
/// returns each function docstring's start mapped to its
/// signature-order names, the mirror key for docstring-entry sorting.
pub(super) fn collect_leaf_edits<'a>(
    source: &'a Source,
    rewrite_targets: &'a HashMap<TextSize, &'a Parameters>,
) -> (Vec<Edit>, HashMap<TextSize, Vec<&'a str>>) {
    let mut collector = LeafCollector {
        edits: Vec::new(),
        param_docs: HashMap::new(),
        rewrite_edits: Vec::new(),
        rewrite_targets,
        scope: BodyScope::Module,
        source,
    };
    collector.visit_body(&source.ast().body);
    let LeafCollector {
        mut edits,
        param_docs,
        rewrite_edits: mut rewrites,
        ..
    } = collector;
    // Keyword rewrites are pure additions over the existing leaf edits,
    // so drop any that would overlap one, sidestepping the leaf-edit
    // applicator's non-overlap invariant on nested reorder spans. An
    // enclosing rewrite outranks one it contains, so widest-first
    // ordering keeps the outer of a nested pair.
    rewrites.sort_by_key(|e| (e.start(), Reverse(e.end())));
    for rewrite in rewrites {
        if edits.iter().all(|e| {
            e.range()
                .intersect(rewrite.range())
                .is_none_or(TextRange::is_empty)
        }) {
            insert_by_start(&mut edits, rewrite);
        }
    }
    (edits, param_docs)
}

/// True when sorting a function's positional-or-keyword `args` by the
/// parameter sort key would change their order.
fn args_reorder(params: &Parameters) -> bool {
    !params.args.iter().filter_map(classify_param).is_sorted()
}

/// Composite parameter sort key. Required parameters (no default)
/// sort before optional parameters (has default), each sub-group by
/// name. `self` and `cls` pin in place.
fn classify_param(p: &ParameterWithDefault) -> Option<(u8, &str)> {
    let name = p.name().as_str();
    if matches!(name, "cls" | "self") {
        return None;
    }
    Some((u8::from(p.default.is_some()), name))
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
/// signature: positional-only and `*args` / `**kwargs` in place,
/// positional-or-keyword in source order when `pinned` and in
/// sort-key order otherwise, keyword-only always in sort-key order.
fn signature_order(params: &Parameters, pinned: bool) -> Vec<&str> {
    let mut names: Vec<&str> = params
        .posonlyargs
        .iter()
        .map(|p| p.name().as_str())
        .collect();
    names.extend(sorted_names(&params.args, !pinned));
    names.extend(params.vararg.as_deref().map(|p| p.name.as_str()));
    names.extend(sorted_names(&params.kwonlyargs, true));
    names.extend(params.kwarg.as_deref().map(|p| p.name.as_str()));
    names
}

/// Returns the names of `params` in source order, or permuted by the
/// parameter sort key when `sorted`.
fn sorted_names(params: &[ParameterWithDefault], sorted: bool) -> Vec<&str> {
    let mut order: Vec<usize> = (0..params.len()).collect();
    if sorted {
        permute_full(&mut order, params, classify_param);
    }
    order.iter().map(|&i| params[i].name().as_str()).collect()
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, first_def, parse};

    #[rstest]
    #[case("def f(b, a): pass\n", true)]
    #[case("def f(a, b): pass\n", false)]
    #[case("def f(a): pass\n", false)]
    #[case("def f(): pass\n", false)]
    #[case("def f(self, b, a): pass\n", true)]
    #[case("def f(b, a, /): pass\n", false)]
    fn args_reorder_tracks_only_the_positional_or_keyword_args(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let s = parse(src);
        let f = first_def(&s);
        assert_eq!(args_reorder(&f.parameters), expected);
    }

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
        @pytest.mark.parametrize('x', [1])
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
    fn collect_docstring_entry_edits_mirrors_pinned_signature_order(#[case] src: &str) {
        let source = parse(src);
        let targets = call_rewrite_targets(&source);
        let (_, param_docs) = collect_leaf_edits(&source, &targets);
        let edits = collect_docstring_entry_edits(&source, &param_docs);
        let text = applied_text(&source, edits);
        let pos = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} present"))
        };
        assert!(
            pos("b: two") < pos("a: one"),
            "parameter entries mirror the pinned signature"
        );
        assert!(
            pos("KeyError: missing") < pos("ValueError: bad"),
            "non-parameter entries still sort"
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
        let targets = call_rewrite_targets(&source);
        let (_, param_docs) = collect_leaf_edits(&source, &targets);
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

    #[test]
    fn collect_leaf_edits_drops_a_keyword_rewrite_overlapping_another_edit() {
        let source = parse(
            "def inner(b, a):\n    pass\n\n\ndef outer(d, c):\n    pass\n\n\nouter(inner(1, 2), 3)\n",
        );
        let (edits, _) = collect_leaf_edits(&source, &call_rewrite_targets(&source));
        let text = applied_text(&source, edits);
        assert_eq!(
            text,
            "def inner(a, b):\n    pass\n\n\ndef outer(c, d):\n    pass\n\n\nouter(c=3, d=inner(1, 2))\n",
        );
    }

    #[rstest]
    #[case(
        "class C:\n    def m(self, b, a): pass\n",
        "class C:\n    def m(self, b, a): pass\n"
    )]
    #[case(
        "class C:\n    if True:\n        def m(self, b, a): pass\n",
        "class C:\n    if True:\n        def m(self, b, a): pass\n"
    )]
    #[case(
        "class C:\n    async def m(self, b, a): pass\n",
        "class C:\n    async def m(self, b, a): pass\n"
    )]
    #[case(
        "class C:\n    class D:\n        def m(self, b, a): pass\n",
        "class C:\n    class D:\n        def m(self, b, a): pass\n"
    )]
    #[case(
        "class C:\n    def m(self, b, a, *, d=1, c=2): pass\n",
        "class C:\n    def m(self, b, a, *, c=2, d=1): pass\n"
    )]
    #[case("def m(b, a): pass\n", "def m(a, b): pass\n")]
    #[case(
        "class C:\n    pass\n\n\ndef m(b, a): pass\n",
        "class C:\n    pass\n\n\ndef m(a, b): pass\n"
    )]
    #[case(
        "class C:\n    def m(self):\n        def inner(b, a): pass\n",
        "class C:\n    def m(self):\n        def inner(a, b): pass\n"
    )]
    #[case(
        "class C:\n    key = lambda b, a: 0\n",
        "class C:\n    key = lambda b, a: 0\n"
    )]
    #[case(
        "class C:\n    def m(self):\n        key = lambda b, a: 0\n",
        "class C:\n    def m(self):\n        key = lambda a, b: 0\n"
    )]
    fn collect_leaf_edits_pins_class_body_function_params(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let (edits, _) = collect_leaf_edits(&source, &call_rewrite_targets(&source));
        assert_eq!(applied_text(&source, edits), expected);
    }

    #[test]
    fn collect_leaf_edits_yields_edits_in_source_order() {
        let src = indoc! {"
            import b, a
            from m import d, c
            __all__ = ['z', 'y']
            x = {z, y}
            def f(b, a): foo(b=2, a=1)
        "};
        let source = parse(src);
        let (edits, _) = collect_leaf_edits(&source, &call_rewrite_targets(&source));
        assert!(edits.len() >= 5, "fixture must trigger multiple producers");
        assert!(
            edits.is_sorted(),
            "leaf edits must be emitted in source order, since partition_point in apply_inline_edits relies on it",
        );
    }
}
