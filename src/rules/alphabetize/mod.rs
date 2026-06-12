//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning. The covered shapes are classes and functions in a body,
//! class-scope `Stmt::AnnAssign` field declarations and `Stmt::Assign`
//! runs with simple `Name` targets, function and lambda parameters
//! with `self` / `cls`, positional-only params, parameters under a
//! positional-binding decorator, and the positional-or-keyword
//! parameters of class-body functions pinned, call kwargs, set literal
//! elements, consecutive `import` blocks reordered into canonical bare
//! / external-`from` / local-package groups plus their alias lists,
//! `global` and `nonlocal` name lists, `del` target lists, and the
//! string literals inside `__all__` / `__slots__`.
//!
//! Sorting flows through `primitives::orderer::reorder_text`. A
//! recursive `Cow<'src, str>` rewriter folds inner sorts into the
//! outer scope's replacement text, so each outermost reordering scope
//! emits a single edit covering its descendants.
//!
//! When a top-level function's positional parameters reorder, every
//! in-module call resolved through `BindingAnalysis` rewrites its
//! keyword-eligible positional arguments to `name=value`, alphabetized,
//! leaving positional-only prefixes and `*` / `**` call sites in place.

use std::{borrow::Cow, cmp::Reverse, collections::HashMap, ops::Range};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Decorator, ExceptHandler, Expr, ExprCall, ExprDict, ExprLambda, ExprSet, Identifier,
    ParameterWithDefault, Parameters, PythonVersion, Stmt, StmtAnnAssign, StmtAssign, StmtDelete,
    StmtFunctionDef,
    helpers::{any_over_expr, is_compound_statement, is_dunder, map_callable},
    visitor::{Visitor as AstVisitor, walk_expr, walk_stmt},
};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        call_keywords::{keyword_args, module_call_params, pins_positional_params},
        docstring::{body_docstring, entry_carrying_sections, rewrite_docstrings},
        edit::{apply_inline_edits, narrowed_replacement, singleton_groups},
        imports::{ImportGroup, future_annotations_alias, import_group},
        orderer::{
            assemble_blocks, block_range, blocks_span, permute_full, permute_in_place, reorder_text,
        },
        scope::BodyScope,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod bands;
mod dict;
mod tiering;

use self::{
    bands::{band_module_constants, banded_gap},
    dict::rewrite_dict_text,
    tiering::permute_defs,
};

pub(crate) struct Alphabetize {
    docstring_entries: bool,
    first_party: Vec<String>,
    target_version: Option<PythonVersion>,
}

impl Alphabetize {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            docstring_entries: config.rules.alphabetize.docstring_entries,
            first_party: config.first_party(),
            target_version: config.target_version,
        }
    }
}

impl Rule for Alphabetize {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let rewrite_targets = call_rewrite_targets(source);
        let (mut leaf_edits, pinned_param_docs) = collect_leaf_edits(source, &rewrite_targets);
        if self.docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source, &pinned_param_docs));
            leaf_edits.sort_unstable();
        }
        let ctx = RewriteCtx {
            defer_annotations: defers_annotations(body),
            first_party: &self.first_party,
            leaf_edits: &leaf_edits,
            source,
            target_version: self.target_version,
        };
        let (body_text, body_span) = rewrite_body(
            ctx,
            body,
            TextRange::up_to(source.text().text_len()),
            BodyScope::Module,
        );
        let edits = match body_text {
            Cow::Borrowed(_) => leaf_edits,
            Cow::Owned(text) => narrowed_replacement(source, body_span, text)
                .into_iter()
                .collect(),
        };
        singleton_groups(edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct LeafCollector<'a> {
    edits: Vec<Edit>,
    pinned_param_docs: HashMap<TextSize, &'a Parameters>,
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
            self.emit_parameters(params, false);
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
        let Expr::Name(callee) = c.func.as_ref() else {
            return false;
        };
        let Some(&params) = self.rewrite_targets.get(&callee.range().start()) else {
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
        self.scope = match stmt {
            Stmt::ClassDef(_) => BodyScope::Class,
            Stmt::FunctionDef(_) => BodyScope::Function,
            // A compound statement's arms inherit the enclosing scope,
            // matching the `rewrite_stmt` recursion.
            _ => enclosing,
        };
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
                if pinned && let Some(lit) = body_docstring(&f.body) {
                    self.pinned_param_docs.insert(lit.start(), &f.parameters);
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

/// Invariant context threaded through the body-rewrite recursion.
#[derive(Clone, Copy)]
struct RewriteCtx<'a> {
    defer_annotations: bool,
    first_party: &'a [String],
    leaf_edits: &'a [Edit],
    source: &'a Source,
    target_version: Option<PythonVersion>,
}

/// Returns the `StmtAnnAssign` and its target name when the target
/// is a single `Name`.
fn ann_assign_with_named_field(stmt: &Stmt) -> Option<(&StmtAnnAssign, &str)> {
    let ann = stmt.as_ann_assign_stmt()?;
    Some((ann, ann.target.as_name_expr()?.id.as_str()))
}

/// True when sorting a function's positional-or-keyword `args` by the
/// parameter sort key would change their order.
fn args_reorder(params: &Parameters) -> bool {
    !params.args.iter().filter_map(classify_param).is_sorted()
}

/// Maps each in-module call's callee offset to the parameters of the
/// top-level function it resolves to, restricted to functions whose
/// positional `args` reorder under alphabetization.
fn call_rewrite_targets(source: &Source) -> HashMap<TextSize, &Parameters> {
    module_call_params(source, |func| args_reorder(&func.parameters))
}

/// Returns the slot ranges of consecutive items whose pairwise
/// neighbors satisfy `adjacent`. Singleton runs drop.
fn chunk_runs<T>(items: &[T], mut adjacent: impl FnMut(&T, &T) -> bool) -> Vec<Range<usize>> {
    let mut start = 0;
    items
        .chunk_by(|a, b| adjacent(a, b))
        .filter_map(|chunk| {
            let end = start + chunk.len();
            let range = (chunk.len() >= 2).then_some(start..end);
            start = end;
            range
        })
        .collect()
}

/// True when a class body has at least two `Stmt::AnnAssign` field
/// declarations and at least one method whose decorator carries
/// positional arguments.
fn class_pins_methods(body: &[Stmt]) -> bool {
    body.iter()
        .filter(|s| ann_assign_with_named_field(s).is_some())
        .nth(1)
        .is_some()
        && body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .any(pins_positional_params)
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

/// Walks every docstring in `source` and emits one edit per
/// entry-carrying Google-style section whose `name: description`
/// entries are out of alphabetical order. In a pinned signature's
/// docstring, a section whose every entry names a parameter keeps
/// source order, mirroring the signature it documents. Each edit
/// replaces the section's entries-span with the reordered text.
/// Returns an empty list when no docstring carries a sortable section.
fn collect_docstring_entry_edits(
    source: &Source,
    pinned_param_docs: &HashMap<TextSize, &Parameters>,
) -> Vec<Edit> {
    rewrite_docstrings(source, |source, lit, edits| {
        let pinned_params = pinned_param_docs.get(&lit.start());
        for entries in entry_carrying_sections(source, lit) {
            if let Some(params) = pinned_params
                && entries.iter().all(|e| params.includes(e.name))
            {
                continue;
            }
            let cow = reorder_text(
                source,
                &entries,
                |entry| Some(entry.name),
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
/// returns each pinned signature's docstring start mapped to its
/// parameters, the skip set for docstring-entry sorting.
fn collect_leaf_edits<'a>(
    source: &'a Source,
    rewrite_targets: &'a HashMap<TextSize, &'a Parameters>,
) -> (Vec<Edit>, HashMap<TextSize, &'a Parameters>) {
    let mut collector = LeafCollector {
        edits: Vec::new(),
        pinned_param_docs: HashMap::new(),
        rewrite_edits: Vec::new(),
        rewrite_targets,
        scope: BodyScope::Module,
        source,
    };
    collector.visit_body(&source.ast().body);
    let pinned_param_docs = collector.pinned_param_docs;
    let mut edits = collector.edits;
    // Keyword rewrites are pure additions over the existing leaf edits,
    // so drop any that would overlap one, sidestepping the leaf-edit
    // applicator's non-overlap invariant on nested reorder spans. An
    // enclosing rewrite outranks one it contains, so widest-first
    // ordering keeps the outer of a nested pair.
    let mut rewrites = collector.rewrite_edits;
    rewrites.sort_by_key(|e| (e.start(), Reverse(e.end())));
    for rewrite in rewrites {
        if edits.iter().all(|e| {
            e.range()
                .intersect(rewrite.range())
                .is_none_or(|i| i.is_empty())
        }) {
            insert_by_start(&mut edits, rewrite);
        }
    }
    (edits, pinned_param_docs)
}

/// Returns one `(body, outer)` pair per sub-body of a compound
/// statement. `outer` carries the enclosing arm's range, which bounds
/// `block_range`'s leading-comment scan for the body's first item.
/// Empty sub-bodies are returned as-is and skipped by the caller.
fn compound_sub_bodies(stmt: &Stmt) -> Vec<(&[Stmt], TextRange)> {
    match stmt {
        Stmt::For(s) => vec![(s.body.as_slice(), s.range), (s.orelse.as_slice(), s.range)],
        Stmt::If(s) => std::iter::once((s.body.as_slice(), s.range))
            .chain(
                s.elif_else_clauses
                    .iter()
                    .map(|c| (c.body.as_slice(), c.range)),
            )
            .collect(),
        Stmt::Match(s) => s
            .cases
            .iter()
            .map(|c| (c.body.as_slice(), c.range))
            .collect(),
        Stmt::Try(s) => std::iter::once((s.body.as_slice(), s.range))
            .chain(
                s.handlers
                    .iter()
                    .map(|ExceptHandler::ExceptHandler(h)| (h.body.as_slice(), h.range)),
            )
            .chain([
                (s.orelse.as_slice(), s.range),
                (s.finalbody.as_slice(), s.range),
            ])
            .collect(),
        Stmt::While(s) => vec![(s.body.as_slice(), s.range), (s.orelse.as_slice(), s.range)],
        Stmt::With(s) => vec![(s.body.as_slice(), s.range)],
        _ => Vec::new(),
    }
}

/// Returns `Cow::Borrowed` of `source.slice(span)` when every part
/// is still a borrow of source, signalling no descendant rewrite
/// fired. Otherwise concatenates the parts into a single owned
/// string covering the same span.
fn concat_or_borrow<'src>(
    parts: &[Cow<'src, str>],
    source: &'src Source,
    span: TextRange,
) -> Cow<'src, str> {
    if parts.iter().all(|p| matches!(p, Cow::Borrowed(_))) {
        return Cow::Borrowed(source.slice(span));
    }
    Cow::Owned(parts.concat())
}

fn decorator_simple_name(decorator: &Decorator) -> Option<&str> {
    match map_callable(&decorator.expression) {
        Expr::Attribute(attr) => Some(attr.attr.as_str()),
        Expr::Name(name) => Some(name.id.as_str()),
        _ => None,
    }
}

/// True when the module carries `from __future__ import annotations`,
/// deferring every annotation's evaluation per PEP 563.
fn defers_annotations(body: &[Stmt]) -> bool {
    body.iter()
        .filter_map(Stmt::as_import_from_stmt)
        .any(|node| future_annotations_alias(node).is_some())
}

/// True when an annotated assignment carries a default, either
/// directly via `= value` or through any nested `Call` in the
/// annotation that carries a `default` or `default_factory` keyword.
fn has_default(ann: &StmtAnnAssign) -> bool {
    ann.value.is_some()
        || any_over_expr(&ann.annotation, |e| {
            e.as_call_expr().is_some_and(|c| {
                c.arguments
                    .keywords
                    .iter()
                    .any(|kw| matches!(kw.arg.as_deref(), Some("default" | "default_factory")))
            })
        })
}

/// True when two adjacent statements in `body` sit on one physical
/// line, joined by `;`. A block-based reorder carries such a statement's
/// `;` separator into its new slot and abuts the displaced sibling, so a
/// body carrying one keeps source order.
fn has_inline_statement_join(source: &Source, body: &[Stmt]) -> bool {
    body.windows(2)
        .any(|pair| !source.contains_line_break(TextRange::new(pair[0].end(), pair[1].start())))
}

/// True when the line containing the dict's opening `{` carries a
/// trailing `# prose: keep` comment.
fn has_keep_marker(source: &Source, dict: &ExprDict) -> bool {
    let line = source.text().full_line_range(dict.range().start());
    source
        .comment_ranges()
        .comments_in_range(line)
        .iter()
        .any(|c| source.slice(c).trim_start_matches('#').trim() == "prose: keep")
}

/// Composite import sort key landing the canonical group order
/// (bare → external `from` → local-package) ahead of a per-kind
/// inner sort. Within a group, bare imports sort before `from`
/// imports, bare by least alias name and `from` by `(level, module)`.
/// `None` pins any non-import statement in place.
fn import_sort_key<'a>(
    stmt: &'a Stmt,
    first_party: &[String],
) -> Option<(ImportGroup, u8, u32, &'a str)> {
    let group = import_group(stmt, first_party)?;
    Some(match stmt {
        Stmt::Import(i) => (group, 0, 0, least_alias(&i.names)),
        Stmt::ImportFrom(i) => (group, 1, i.level, i.module.as_deref().unwrap_or_default()),
        _ => unreachable!("import_group returns Some only for import statements"),
    })
}

/// Inserts `edit` into a `Vec<Edit>` kept sorted by `range().start()`,
/// preserving that order.
fn insert_by_start(edits: &mut Vec<Edit>, edit: Edit) {
    let slot = edits.partition_point(|e| e.range().start() < edit.range().start());
    edits.insert(slot, edit);
}

/// Returns the alphabetically least alias name in a bare import's
/// name list. An `import` statement always binds at least one name.
fn least_alias(names: &[Alias]) -> &str {
    names
        .iter()
        .map(|a| a.name.as_str())
        .min()
        .expect("import binds at least one name")
}

/// Returns the method-group index. `0` for dunders, `1` for
/// `@property` / `@cached_property` (decided by the first decorator),
/// `2` for single-leading-underscore privates, `3` for public.
fn method_group(f: &StmtFunctionDef) -> u8 {
    let name = f.name.as_str();
    if is_dunder(name) {
        0
    } else if f
        .decorator_list
        .first()
        .and_then(decorator_simple_name)
        .is_some_and(|n| matches!(n, "cached_property" | "property"))
    {
        1
    } else if name.starts_with('_') {
        2
    } else {
        3
    }
}

/// Rewrites a non-empty body, returning the rewritten text alongside
/// the block-extent span it covers. The text is `Cow::Owned` when any
/// sibling reorder fires, any descendant rewrite produces owned
/// content, or any leaf edit lands inside, falling back to
/// `Cow::Borrowed` over `source.slice(span)`. `scope` selects which
/// family sorts apply.
fn rewrite_body<'a>(
    ctx: RewriteCtx<'a>,
    body: &[Stmt],
    outer: TextRange,
    scope: BodyScope,
) -> (Cow<'a, str>, TextRange) {
    let RewriteCtx {
        defer_annotations,
        first_party,
        source,
        target_version,
        ..
    } = ctx;
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'a, str>>) = body
        .iter()
        .enumerate()
        .map(|(i, stmt)| {
            let block = block_range(source, body, i, outer);
            (block, rewrite_stmt(ctx, stmt, block, scope))
        })
        .unzip();
    let body_span = blocks_span(&blocks);
    let n = body.len();
    let mut order: Vec<usize> = (0..n).collect();
    let mut import_run_slots: Vec<usize> = Vec::new();
    let mut band_ranks: Option<HashMap<usize, u8>> = None;
    if !has_inline_statement_join(source, body) {
        let in_class = scope == BodyScope::Class;
        if scope != BodyScope::Function {
            permute_defs(&mut order, body, defer_annotations, |s| {
                s.as_class_def_stmt().map(|c| {
                    let name = c.name.as_str();
                    (name, name)
                })
            });
            if in_class {
                permute_full(&mut order, body, |s| {
                    ann_assign_with_named_field(s)
                        .map(|(ann, name)| (u8::from(has_default(ann)), name))
                });
                permute_full(&mut order, body, simple_name_assign);
            }
            if !(in_class && class_pins_methods(body)) {
                permute_defs(&mut order, body, defer_annotations, |s| {
                    s.as_function_def_stmt().map(|f| {
                        let name = f.name.as_str();
                        (name, (method_group(f), name))
                    })
                });
            }
        }
        let is_import = |s: &Stmt| s.is_import_stmt() || s.is_import_from_stmt();
        for Range { start, end } in statement_run_ranges(body, is_import) {
            permute_in_place(&mut order, body, start..end, |s| {
                import_sort_key(s, first_party)
            });
        }
        if scope == BodyScope::Module {
            band_ranks = band_module_constants(
                source,
                body,
                &blocks,
                defer_annotations,
                target_version,
                &mut order,
            );
        }
        // A banded order reconstructs its own blank-line texture. Otherwise
        // same-group import neighbors collapse to one line, derived from the
        // assembled order the family sorts left.
        if band_ranks.is_none() {
            let group = |slot: usize| import_group(&body[order[slot]], first_party);
            import_run_slots.extend(
                (0..n.saturating_sub(1))
                    .filter(|&slot| group(slot).is_some() && group(slot) == group(slot + 1)),
            );
        }
    }
    let any_owned = rendered.iter().any(|c| matches!(c, Cow::Owned(_)));
    let identity = order.iter().copied().eq(0..n);
    if !any_owned && identity && import_run_slots.is_empty() {
        return (Cow::Borrowed(source.slice(body_span)), body_span);
    }
    let assembled = assemble_blocks(source, &blocks, &rendered, &order, |i| match &band_ranks {
        Some(ranks) => banded_gap(ranks, body, first_party, order[i], order[i + 1]),
        None => import_run_slots.binary_search(&i).is_ok().then_some("\n"),
    });
    (Cow::Owned(assembled), body_span)
}

/// Recurses into each sub-body of a compound statement, splicing
/// rewritten bodies back into the parent block while leaving header,
/// keyword, and inter-arm regions to leaf-level edits.
fn rewrite_compound<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &Stmt,
    block: TextRange,
    scope: BodyScope,
) -> Cow<'a, str> {
    let bodies = compound_sub_bodies(stmt)
        .into_iter()
        .filter(|(body, _)| !body.is_empty())
        .map(|(body, outer)| rewrite_body(ctx, body, outer, scope));
    splice_bodies(ctx.source, block, bodies, ctx.leaf_edits)
}

/// Rewrites a single statement. Classes and functions fold their body
/// via `rewrite_body` and splice the result. Compound statements
/// (`if`, `for`, `while`, `with`, `try`, `match`) recurse into each
/// sub-body with the inherited `parent_scope`, so module-level reorders
/// (imports, classes, top-level functions) fire inside `if TYPE_CHECKING`
/// and other body-bearing arms. Other shapes apply leaf edits in place.
fn rewrite_stmt<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &Stmt,
    block: TextRange,
    parent_scope: BodyScope,
) -> Cow<'a, str> {
    let (body, body_outer, scope): (&[Stmt], TextRange, BodyScope) = match stmt {
        Stmt::ClassDef(c) => (&c.body, c.range(), BodyScope::Class),
        Stmt::FunctionDef(f) => (&f.body, f.range(), BodyScope::Function),
        s if is_compound_statement(s) => {
            return rewrite_compound(ctx, stmt, block, parent_scope);
        }
        _ => return apply_inline_edits(ctx.source, block, ctx.leaf_edits),
    };
    if body.is_empty() {
        return apply_inline_edits(ctx.source, block, ctx.leaf_edits);
    }
    let (body_text, body_span) = rewrite_body(ctx, body, body_outer, scope);
    splice_bodies(ctx.source, block, [(body_text, body_span)], ctx.leaf_edits)
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

/// Returns the simple name assigned by an `Stmt::Assign` whose
/// target is a single `Name`. `None` for multi-target,
/// destructuring, attribute, or subscript targets.
fn simple_name_assign(stmt: &Stmt) -> Option<&str> {
    match stmt.as_assign_stmt()?.targets.as_slice() {
        [Expr::Name(name)] => Some(name.id.as_str()),
        _ => None,
    }
}

/// Splices `bodies` back into `block`, folding leaf edits into the
/// pre-, inter-, and post-body gaps. `bodies` must be in source
/// order.
fn splice_bodies<'src, I>(
    source: &'src Source,
    block: TextRange,
    bodies: I,
    leaf_edits: &[Edit],
) -> Cow<'src, str>
where
    I: IntoIterator<Item = (Cow<'src, str>, TextRange)>,
{
    let mut parts = Vec::new();
    let mut cursor = block.start();
    for (text, span) in bodies {
        parts.push(apply_inline_edits(
            source,
            TextRange::new(cursor, span.start()),
            leaf_edits,
        ));
        parts.push(text);
        cursor = span.end();
    }
    parts.push(apply_inline_edits(
        source,
        TextRange::new(cursor, block.end()),
        leaf_edits,
    ));
    concat_or_borrow(&parts, source, block)
}

/// Builds a `Vec<Range<usize>>` of body slots whose statements all
/// match `predicate`. Consecutive matching slots collapse into one
/// run, and a non-matching statement between two matching ones breaks
/// the run. Singleton runs drop.
fn statement_run_ranges(
    body: &[Stmt],
    mut predicate: impl FnMut(&Stmt) -> bool,
) -> Vec<Range<usize>> {
    chunk_runs(body, |a, b| predicate(a) && predicate(b))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    fn applied_text(source: &Source, edits: Vec<Edit>) -> String {
        crate::primitives::edit::apply_edits(source.text(), edits).expect("non-overlapping edits")
    }

    #[test]
    fn ann_assign_with_named_field_filters_to_name_targets() {
        let s = parse("x: int = 1\nself.x: int = 1\n");
        let names: Vec<Option<&str>> = s
            .ast()
            .body
            .iter()
            .map(|s| ann_assign_with_named_field(s).map(|(_, name)| name))
            .collect();
        assert_eq!(names, vec![Some("x"), None]);
    }

    #[test]
    fn apply_skips_docstring_entry_reorder_when_config_disables_it() {
        let src = indoc! {"
            def f():
                \"\"\"Summary.

                Args:
                    bar: two
                    alpha: one
                \"\"\"
                pass
        "};
        let mut config = Config::default();
        config.rules.alphabetize.docstring_entries = false;
        let rule = Alphabetize::from_config(&config);
        let source = parse(src);
        let edits = rule.apply(&source).into_iter().flatten().collect();
        let text = applied_text(&source, edits);
        let args_section_end = text.find("\"\"\"\n    pass").expect("closer follows args");
        let args_section = &text[..args_section_end];
        let bar_pos = args_section.find("bar: two").expect("bar still present");
        let alpha_pos = args_section
            .find("alpha: one")
            .expect("alpha still present");
        assert!(
            bar_pos < alpha_pos,
            "docstring entries should keep source order when docstring-entries is off",
        );
    }

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
        let f = s.ast().body[0].as_function_def_stmt().expect("def");
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
    fn collect_docstring_entry_edits_pins_param_sections_of_pinned_signatures(#[case] src: &str) {
        let source = parse(src);
        let targets = call_rewrite_targets(&source);
        let (_, pinned) = collect_leaf_edits(&source, &targets);
        let edits = collect_docstring_entry_edits(&source, &pinned);
        let text = applied_text(&source, edits);
        let b = text.find("b: two").expect("b entry present");
        let a = text.find("a: one").expect("a entry present");
        let value_error = text
            .find("ValueError: bad")
            .expect("ValueError entry present");
        let key_error = text
            .find("KeyError: missing")
            .expect("KeyError entry present");
        assert!(b < a, "parameter entries keep source order");
        assert!(key_error < value_error, "non-parameter entries still sort");
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
        "class C:\n    key = lambda a, b: 0\n"
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

    #[rstest]
    #[case("@property\ndef f(): pass\n", Some("property"))]
    #[case("@functools.cached_property\ndef f(): pass\n", Some("cached_property"))]
    #[case("@click.option(\"--name\")\ndef f(): pass\n", Some("option"))]
    #[case(
        "@pytest.mark.parametrize(\"a\", [1])\ndef f(): pass\n",
        Some("parametrize")
    )]
    #[case("@functools.wraps(other)\ndef f(): pass\n", Some("wraps"))]
    fn decorator_simple_name_extracts_rightmost_segment(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let s = parse(src);
        let f = s.ast().body[0].as_function_def_stmt().expect("def");
        let decorator = f.decorator_list.first().expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), expected);
    }

    #[test]
    fn decorator_simple_name_returns_none_for_complex_expressions() {
        let s = parse("@(some_factory())()\ndef f(): pass\n");
        let f = s.ast().body[0].as_function_def_stmt().expect("def");
        let decorator = f.decorator_list.first().expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), None);
    }

    #[rstest]
    #[case("from __future__ import annotations\n", true)]
    #[case("from __future__ import annotations, division\n", true)]
    #[case("from __future__ import division\n", false)]
    #[case("from other import annotations\n", false)]
    #[case("import __future__\n", false)]
    #[case("x = 1\n", false)]
    fn defers_annotations_detects_the_future_import(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        assert_eq!(defers_annotations(&source.ast().body), expected);
    }

    #[rstest]
    #[case("import b\nimport a; x = 1\n", true)]
    #[case("import b\nimport a\n", false)]
    #[case("a = 1; b = 2\n", true)]
    #[case("x = 1\n", false)]
    fn has_inline_statement_join_detects_semicolon_joined_siblings(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        assert_eq!(
            has_inline_statement_join(&source, &source.ast().body),
            expected
        );
    }

    #[test]
    fn import_sort_key_ranks_groups_then_bare_before_from_within_local() {
        let first_party = vec!["myapp".to_owned()];
        let s = parse("import os\nfrom os import path\nimport myapp.core\nfrom myapp import app\n");
        let keys: Vec<_> = s
            .ast()
            .body
            .iter()
            .map(|stmt| import_sort_key(stmt, &first_party).expect("import statement"))
            .collect();
        assert!(
            keys[0] < keys[1] && keys[1] < keys[2] && keys[2] < keys[3],
            "expected bare-external < external-from < local-bare < local-from",
        );
    }

    #[test]
    fn import_sort_key_returns_none_for_non_import() {
        let s = parse("x = 1\n");
        assert!(import_sort_key(&s.ast().body[0], &[]).is_none());
    }

    #[test]
    fn least_alias_returns_alphabetically_min_name() {
        let s = parse("import sys, os, abc\n");
        let import = s.ast().body[0].as_import_stmt().expect("import");
        assert_eq!(least_alias(&import.names), "abc");
    }

    #[test]
    fn method_group_orders_dunder_property_private_public() {
        let src = indoc! {"
            class C:
                def __init__(self): pass
                @property
                def name(self): pass
                def _helper(self): pass
                def public(self): pass
        "};
        let s = parse(src);
        let class = s.ast().body[0].as_class_def_stmt().expect("class");
        let groups: Vec<u8> = class
            .body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .map(method_group)
            .collect();
        assert_eq!(groups, vec![0, 1, 2, 3]);
    }

    #[test]
    fn simple_name_assign_filters_to_single_name_targets() {
        let s = parse("X = 1\nself.x = 1\nx, y = 1, 2\n");
        let names: Vec<Option<&str>> = s.ast().body.iter().map(simple_name_assign).collect();
        assert_eq!(names, vec![Some("X"), None, None]);
    }
}
