//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning. The covered shapes are classes and functions in a body,
//! class-scope `Stmt::AnnAssign` field declarations and `Stmt::Assign`
//! runs with simple `Name` targets, function and lambda parameters
//! with `self` / `cls`, positional-only params, and parameters under a
//! positional-binding decorator pinned, call kwargs, set literal
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

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Decorator, DictItem, ExceptHandler, Expr, ExprCall, ExprDict, ExprLambda, ExprSet,
    Identifier, ParameterWithDefault, Parameters, Stmt, StmtAnnAssign, StmtAssign, StmtDelete,
    StmtFunctionDef,
    helpers::{any_over_expr, is_compound_statement, is_dunder, map_callable},
    visitor::{Visitor as AstVisitor, walk_expr, walk_parameters, walk_stmt},
};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        call_keywords::{keyword_args, module_call_params, pins_positional_params},
        docstring::{entry_carrying_sections, rewrite_docstrings},
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

pub(crate) struct Alphabetize {
    docstring_entries: bool,
    first_party: Vec<String>,
}

impl Alphabetize {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            docstring_entries: config.rules.alphabetize.docstring_entries,
            first_party: config.first_party(),
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
        let mut leaf_edits = collect_leaf_edits(source, &rewrite_targets);
        if self.docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source));
            leaf_edits.sort_unstable();
        }
        let (body_text, body_span) = rewrite_body(
            source,
            body,
            TextRange::up_to(source.text().text_len()),
            BodyScope::Module,
            &leaf_edits,
            &self.first_party,
            defers_annotations(body),
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

/// Collects every `Name` load in a definition's evaluation-time
/// surface: its decorators, base classes and class keywords, parameter
/// defaults, non-deferred annotations, and the top level of a class
/// body, descending into nested definitions but pruning every function
/// and lambda body. Annotation positions are skipped when
/// `defer_annotations` holds.
struct EvalRefVisitor<'src> {
    defer_annotations: bool,
    names: Vec<&'src str>,
}

impl<'src> AstVisitor<'src> for EvalRefVisitor<'src> {
    fn visit_annotation(&mut self, annotation: &'src Expr) {
        if !self.defer_annotations {
            self.visit_expr(annotation);
        }
    }

    fn visit_expr(&mut self, expr: &'src Expr) {
        match expr {
            Expr::Lambda(lambda) => {
                if let Some(params) = lambda.parameters.as_deref() {
                    walk_parameters(self, params);
                }
            }
            Expr::Name(name) if name.ctx.is_load() => self.names.push(name.id.as_str()),
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::AnnAssign(ann) => {
                self.visit_annotation(&ann.annotation);
                if let Some(value) = &ann.value {
                    self.visit_expr(value);
                }
            }
            Stmt::FunctionDef(func) => {
                for decorator in &func.decorator_list {
                    self.visit_expr(&decorator.expression);
                }
                walk_parameters(self, &func.parameters);
                if let Some(returns) = &func.returns {
                    self.visit_annotation(returns);
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

struct LeafCollector<'a> {
    dict_depth: u32,
    edits: Vec<Edit>,
    rewrite_edits: Vec<Edit>,
    rewrite_targets: &'a HashMap<TextSize, &'a Parameters>,
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
        if let Some((span, text)) = rewrite_dict_text(self.source, d) {
            self.edits
                .push(Edit::range_replacement(text.into_owned(), span));
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
        let rendered = reorder_text(self.source, items, classify, |_, slice| {
            Cow::Borrowed(slice)
        });
        if let Cow::Owned(text) = rendered {
            self.edits.push(Edit::range_replacement(text, span));
        }
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
        // nothing to convert; the plain keyword reorder sorts it instead.
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
        if let Expr::Dict(d) = expr {
            if self.dict_depth == 0 {
                self.emit_dict(d);
            }
            self.dict_depth += 1;
            walk_expr(self, expr);
            self.dict_depth -= 1;
            return;
        }
        match expr {
            Expr::Call(c) => self.emit_call(c),
            Expr::Lambda(l) => self.emit_lambda(l),
            Expr::Set(s) => self.emit_set(s),
            _ => {}
        }
        walk_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assign(a) => self.emit_dunder_list(a),
            Stmt::Delete(d) => self.emit_delete(d),
            Stmt::FunctionDef(f) => self.emit_parameters(&f.parameters, pins_positional_params(f)),
            Stmt::Global(g) => self.emit_id_run(&g.names),
            Stmt::Import(i) => self.emit_alias_run(&i.names),
            Stmt::ImportFrom(i) => self.emit_alias_run(&i.names),
            Stmt::Nonlocal(n) => self.emit_id_run(&n.names),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

#[derive(Default)]
struct RhsAnalyzer<'src> {
    access_roots: Vec<&'src str>,
    deps: Vec<&'src str>,
    tainted: bool,
}

impl<'src> AstVisitor<'src> for RhsAnalyzer<'src> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        if self.tainted {
            return;
        }
        match expr {
            Expr::Attribute(_) | Expr::Subscript(_) => {
                let Some(root) = root_name(expr) else {
                    self.tainted = true;
                    return;
                };
                self.access_roots.push(root);
                walk_expr(self, expr);
            }
            Expr::Await(_) | Expr::Call(_) | Expr::Yield(_) | Expr::YieldFrom(_) => {
                self.tainted = true;
            }
            Expr::Name(name) if name.ctx.is_load() => self.deps.push(name.id.as_str()),
            _ => walk_expr(self, expr),
        }
    }
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

/// Concatenates dict-item block texts in `order`, normalizing trailing
/// commas so non-last slots always have one and the new-last slot
/// matches `source_last_has_comma`. Inserts a blank line at every
/// slot listed in `divider_slots`.
fn assemble_dict_items_multiline(
    block_texts: &[Cow<'_, str>],
    order: &[usize],
    divider_slots: &[usize],
    source_last_has_comma: bool,
) -> String {
    let mut out = String::new();
    for (slot, &idx) in order.iter().enumerate() {
        let text = block_texts[idx].trim_end_matches(',');
        out.push_str(text);
        let is_last = slot + 1 == order.len();
        if !is_last || source_last_has_comma {
            out.push(',');
        }
        if !is_last {
            out.push('\n');
            if divider_slots.contains(&slot) {
                out.push('\n');
            }
        }
    }
    out
}

/// Returns the target name and optional value of an `Assign` or
/// `AnnAssign` whose target is a single `Name`. `None` for any other
/// shape.
fn assign_run_target(stmt: &Stmt) -> Option<(&str, Option<&Expr>)> {
    match stmt {
        Stmt::AnnAssign(a) => Some((a.target.as_name_expr()?.id.as_str(), a.value.as_deref())),
        Stmt::Assign(a) => {
            let [Expr::Name(name)] = a.targets.as_slice() else {
                return None;
            };
            Some((name.id.as_str(), Some(a.value.as_ref())))
        }
        _ => None,
    }
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
/// entries are out of alphabetical order. Each edit replaces the
/// section's entries-span with the reordered text. Returns an empty
/// list when no docstring carries a sortable section.
fn collect_docstring_entry_edits(source: &Source) -> Vec<Edit> {
    rewrite_docstrings(source, |source, lit, edits| {
        for entries in entry_carrying_sections(source, lit) {
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

/// Walks the AST collecting every leaf-level edit in source order, then
/// folds in each call-site keyword rewrite that does not overlap an
/// existing edit, keeping the whole list non-overlapping.
fn collect_leaf_edits<'a>(
    source: &'a Source,
    rewrite_targets: &'a HashMap<TextSize, &'a Parameters>,
) -> Vec<Edit> {
    let mut collector = LeafCollector {
        dict_depth: 0,
        edits: Vec::new(),
        rewrite_edits: Vec::new(),
        rewrite_targets,
        source,
    };
    collector.visit_body(&source.ast().body);
    let mut edits = collector.edits;
    edits.sort_unstable();
    // Keyword rewrites are pure additions over the existing leaf edits,
    // so drop any that would overlap one, sidestepping the leaf-edit
    // applicator's non-overlap invariant on nested reorder spans.
    for rewrite in collector.rewrite_edits {
        if edits.iter().all(|e| {
            e.range()
                .intersect(rewrite.range())
                .is_none_or(|i| i.is_empty())
        }) {
            let slot = edits.partition_point(|e| e.range().start() < rewrite.range().start());
            edits.insert(slot, rewrite);
        }
    }
    edits
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

/// Returns a per-member `(tier, key)` lookup keyed by each definition's
/// start offset, or `None` when the run cannot reorder. The run skips
/// when two members share a name or when the intra-run reference graph
/// carries a cycle. A member depends on every other sibling it names in
/// its evaluation-time surface, and the composite `(tier, key)` combines
/// a Kahn-style topological tier with the member's existing sort key, so
/// a definition never sorts ahead of a sibling it names at evaluation
/// time.
fn def_run_tier_keys<'src, K: Copy>(
    body: &'src [Stmt],
    defer_annotations: bool,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
) -> Option<HashMap<TextSize, (usize, K)>> {
    let members: Vec<(&'src Stmt, &'src str, K)> = body
        .iter()
        .filter_map(|stmt| member(stmt).map(|(name, key)| (stmt, name, key)))
        .collect();
    let name_to_idx = unique_name_index(members.iter().map(|&(_, name, _)| name))?;
    let dep_sets: Vec<HashSet<usize>> = members
        .iter()
        .enumerate()
        .map(|(idx, &(stmt, _, _))| {
            let mut visitor = EvalRefVisitor {
                defer_annotations,
                names: Vec::new(),
            };
            visitor.visit_stmt(stmt);
            visitor
                .names
                .iter()
                .filter_map(|name| name_to_idx.get(name).copied())
                // A recursive self-reference does not constrain sibling order.
                .filter(|&dep| dep != idx)
                .collect()
        })
        .collect();
    let tiers = tier_levels(&dep_sets)?;
    Some(
        members
            .iter()
            .zip(tiers)
            .map(|(&(stmt, _, key), tier)| (stmt.range().start(), (tier, key)))
            .collect(),
    )
}

/// True when the module carries `from __future__ import annotations`,
/// deferring every annotation's evaluation per PEP 563.
fn defers_annotations(body: &[Stmt]) -> bool {
    body.iter()
        .filter_map(Stmt::as_import_from_stmt)
        .any(|node| future_annotations_alias(node).is_some())
}

/// Composite dict-item sort key. `**unpacked` items return `None` and
/// pin in source position. Keyed items sort single-line entries before
/// multi-line entries and alphabetize within each partition by the
/// key's source slice.
fn dict_sort_key<'a>(source: &'a Source, item: &'a DictItem) -> Option<(u8, &'a str)> {
    let key = item.key.as_ref()?;
    let group = u8::from(source.contains_line_break(item.range()));
    Some((group, source.slice(key)))
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

/// Returns the paragraph-adjacent runs of `Assign` and `AnnAssign`
/// statements in `body`. A blank line (two or more newlines in the
/// gap) breaks the run. Trailing comments on assign lines do not
/// break the run on their own.
fn module_assign_run_ranges(source: &Source, body: &[Stmt]) -> Vec<Range<usize>> {
    let is_assign = |s: &Stmt| s.is_assign_stmt() || s.is_ann_assign_stmt();
    chunk_runs(body, |a, b| {
        is_assign(a) && is_assign(b) && {
            let gap = TextRange::new(a.end(), b.range().start());
            source.slice(gap).matches('\n').count() <= 1
        }
    })
}

/// Returns a per-run tier-and-name lookup keyed by each statement's
/// start offset, or `None` when the run cannot reorder. The run skips
/// when any item has a non-`Name` target, duplicates an earlier
/// target, carries a side-effecting RHS shape (`Call`, `Await`,
/// `Yield`, `YieldFrom`, or an `Attribute` / `Subscript` whose root
/// names neither a run target nor a module-local binding written
/// before the run), or when the intra-run dependency graph carries a
/// cycle. The composite `(tier, name)` key combines a Kahn-style
/// topological tier with the binding's name.
fn module_assign_tier_keys<'src>(
    source: &'src Source,
    run: &'src [Stmt],
) -> Option<HashMap<TextSize, (usize, &'src str)>> {
    let first = run.first()?;
    let line_extent = TextRange::new(
        source.text().line_start(first.range().start()),
        source.text().full_line_end(run.last()?.range().end()),
    );
    if source.intersects_comment(line_extent) {
        return None;
    }
    let extracted: Vec<(&'src str, Option<&'src Expr>)> =
        run.iter().map(assign_run_target).collect::<Option<_>>()?;
    let target_to_idx = unique_name_index(extracted.iter().map(|&(name, _)| name))?;
    let analysis = source.binding_analysis();
    let run_start = first.range().start();
    let dep_sets: Vec<HashSet<usize>> = extracted
        .iter()
        .map(|&(_, value)| -> Option<HashSet<usize>> {
            let mut analyzer = RhsAnalyzer::default();
            if let Some(rhs) = value {
                analyzer.visit_expr(rhs);
            }
            if analyzer.tainted
                || analyzer.access_roots.iter().any(|&root| {
                    !target_to_idx.contains_key(root)
                        && !analysis.is_defined_before(root, run_start)
                })
            {
                return None;
            }
            Some(
                analyzer
                    .deps
                    .iter()
                    .filter_map(|name| target_to_idx.get(name).copied())
                    .collect(),
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let tiers = tier_levels(&dep_sets)?;
    Some(
        run.iter()
            .zip(tiers)
            .zip(&extracted)
            .map(|((stmt, tier), &(name, _))| (stmt.range().start(), (tier, name)))
            .collect(),
    )
}

/// Returns the new-order slot indices after which a blank-line
/// divider should sit. A divider goes on either side of every keyed
/// multi-line entry, isolating it from its neighbors so each
/// multi-line entry forms its own alignment group downstream.
fn partition_divider_slots(source: &Source, order: &[usize], items: &[DictItem]) -> Vec<usize> {
    let is_multiline =
        |i: usize| items[i].key.is_some() && source.contains_line_break(items[i].range());
    order
        .windows(2)
        .enumerate()
        .filter(|(_, w)| is_multiline(w[0]) || is_multiline(w[1]))
        .map(|(i, _)| i)
        .collect()
}

/// Tiers the definition run selected by `member` and permutes `order`
/// by `(tier, key)`, leaving `order` untouched when the run declines.
fn permute_defs<'src, K: Copy + Ord>(
    order: &mut [usize],
    body: &'src [Stmt],
    defer_annotations: bool,
    member: impl Fn(&'src Stmt) -> Option<(&'src str, K)>,
) {
    if let Some(keys) = def_run_tier_keys(body, defer_annotations, member) {
        permute_full(order, body, |s| keys.get(&s.range().start()).copied());
    }
}

/// Rewrites a non-empty body, returning the rewritten text alongside
/// the block-extent span it covers. The text is `Cow::Owned` when any
/// sibling reorder fires, any descendant rewrite produces owned
/// content, or any leaf edit lands inside, falling back to
/// `Cow::Borrowed` over `source.slice(span)`. `scope` selects which
/// family sorts apply.
fn rewrite_body<'src>(
    source: &'src Source,
    body: &[Stmt],
    outer: TextRange,
    scope: BodyScope,
    leaf_edits: &[Edit],
    first_party: &[String],
    defer_annotations: bool,
) -> (Cow<'src, str>, TextRange) {
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'src, str>>) = body
        .iter()
        .enumerate()
        .map(|(i, stmt)| {
            let block = block_range(source, body, i, outer);
            (
                block,
                rewrite_stmt(
                    source,
                    stmt,
                    block,
                    scope,
                    leaf_edits,
                    first_party,
                    defer_annotations,
                ),
            )
        })
        .unzip();
    let body_span = blocks_span(&blocks);
    let n = body.len();
    let mut order: Vec<usize> = (0..n).collect();
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
                ann_assign_with_named_field(s).map(|(ann, name)| (u8::from(has_default(ann)), name))
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
    let mut import_run_slots: Vec<usize> = Vec::new();
    let is_import = |s: &Stmt| s.is_import_stmt() || s.is_import_from_stmt();
    for Range { start, end } in statement_run_ranges(body, is_import) {
        permute_in_place(&mut order, body, start..end, |s| {
            import_sort_key(s, first_party)
        });
        import_run_slots.extend((start..end - 1).filter(|&slot| {
            import_group(&body[order[slot]], first_party)
                == import_group(&body[order[slot + 1]], first_party)
        }));
    }
    if scope == BodyScope::Module {
        for Range { start, end } in module_assign_run_ranges(source, body) {
            let Some(keys) = module_assign_tier_keys(source, &body[start..end]) else {
                continue;
            };
            permute_in_place(&mut order, body, start..end, |s| {
                keys.get(&s.range().start()).copied()
            });
        }
    }
    let any_owned = rendered.iter().any(|c| matches!(c, Cow::Owned(_)));
    let identity = order.iter().copied().eq(0..n);
    if !any_owned && identity && import_run_slots.is_empty() {
        return (Cow::Borrowed(source.slice(body_span)), body_span);
    }
    let assembled = assemble_blocks(source, &blocks, &rendered, &order, |i| {
        import_run_slots.binary_search(&i).is_ok().then_some("\n")
    });
    (Cow::Owned(assembled), body_span)
}

/// Recurses into each sub-body of a compound statement, splicing
/// rewritten bodies back into the parent block while leaving header,
/// keyword, and inter-arm regions to leaf-level edits.
fn rewrite_compound<'src>(
    source: &'src Source,
    stmt: &Stmt,
    block: TextRange,
    scope: BodyScope,
    leaf_edits: &[Edit],
    first_party: &[String],
    defer_annotations: bool,
) -> Cow<'src, str> {
    let bodies = compound_sub_bodies(stmt)
        .into_iter()
        .filter(|(body, _)| !body.is_empty())
        .map(|(body, outer)| {
            rewrite_body(
                source,
                body,
                outer,
                scope,
                leaf_edits,
                first_party,
                defer_annotations,
            )
        });
    splice_bodies(source, block, bodies, leaf_edits)
}

/// Rewrites a dict literal's items span. Returns `Some((span, text))`
/// when reordering, partition, or any nested-dict rewrite produces
/// text different from the source slice. Returns `None` for empty
/// dicts, dicts marked `# prose: keep`, single-item dicts, and any
/// already-canonical case. Recurses into nested dicts that sit
/// directly as item values.
fn rewrite_dict_text<'src>(
    source: &'src Source,
    d: &ExprDict,
) -> Option<(TextRange, Cow<'src, str>)> {
    if d.is_empty() || has_keep_marker(source, d) {
        return None;
    }
    let [first, .., last] = d.items.as_slice() else {
        return None;
    };
    let multi_line = source.contains_line_break(first.range().cover(last.range()));
    let blocks: Vec<TextRange> = if multi_line {
        (0..d.len())
            .map(|i| block_range(source, &d.items, i, d.range()))
            .collect()
    } else {
        d.iter().map(Ranged::range).collect()
    };
    let span = blocks_span(&blocks);
    let block_texts: Vec<Cow<'src, str>> = blocks
        .iter()
        .zip(d)
        .map(|(&block, item)| rewrite_item_block(source, block, item))
        .collect();
    let any_nested_rewrite = block_texts.iter().any(|c| matches!(c, Cow::Owned(_)));
    let mut order: Vec<usize> = (0..d.len()).collect();
    let permuted = permute_full(&mut order, &d.items, |item| dict_sort_key(source, item));
    let assembled = if multi_line {
        let divider_slots = partition_divider_slots(source, &order, &d.items);
        let source_last_has_comma = source
            .slice(*blocks.last().expect("non-empty"))
            .trim_end()
            .ends_with(',');
        assemble_dict_items_multiline(&block_texts, &order, &divider_slots, source_last_has_comma)
    } else {
        assemble_blocks(source, &blocks, &block_texts, &order, |_| None)
    };
    if !permuted && !any_nested_rewrite && assembled == source.slice(span) {
        return None;
    }
    Some((span, Cow::Owned(assembled)))
}

/// Returns the block text for a dict item, recursively rewriting a
/// nested dict that sits directly as the item's value. Returns
/// `Cow::Borrowed` of `source.slice(block)` when no recursion fires or
/// the recursive call leaves the inner dict unchanged.
fn rewrite_item_block<'src>(
    source: &'src Source,
    block: TextRange,
    item: &DictItem,
) -> Cow<'src, str> {
    let Some(inner) = item.value.as_dict_expr() else {
        return Cow::Borrowed(source.slice(block));
    };
    let Some((inner_span, inner_text)) = rewrite_dict_text(source, inner) else {
        return Cow::Borrowed(source.slice(block));
    };
    let prefix = source.slice(TextRange::new(block.start(), inner_span.start()));
    let suffix = source.slice(TextRange::new(inner_span.end(), block.end()));
    Cow::Owned(format!("{prefix}{inner_text}{suffix}"))
}

/// Rewrites a single statement. Classes and functions fold their body
/// via `rewrite_body` and splice the result. Compound statements
/// (`if`, `for`, `while`, `with`, `try`, `match`) recurse into each
/// sub-body with the inherited `parent_scope`, so module-level reorders
/// (imports, classes, top-level functions) fire inside `if TYPE_CHECKING`
/// and other body-bearing arms. Other shapes apply leaf edits in place.
fn rewrite_stmt<'src>(
    source: &'src Source,
    stmt: &Stmt,
    block: TextRange,
    parent_scope: BodyScope,
    leaf_edits: &[Edit],
    first_party: &[String],
    defer_annotations: bool,
) -> Cow<'src, str> {
    let (body, body_outer, scope): (&[Stmt], TextRange, BodyScope) = match stmt {
        Stmt::ClassDef(c) => (&c.body, c.range(), BodyScope::Class),
        Stmt::FunctionDef(f) => (&f.body, f.range(), BodyScope::Function),
        s if is_compound_statement(s) => {
            return rewrite_compound(
                source,
                stmt,
                block,
                parent_scope,
                leaf_edits,
                first_party,
                defer_annotations,
            );
        }
        _ => return apply_inline_edits(source, block, leaf_edits),
    };
    if body.is_empty() {
        return apply_inline_edits(source, block, leaf_edits);
    }
    let (body_text, body_span) = rewrite_body(
        source,
        body,
        body_outer,
        scope,
        leaf_edits,
        first_party,
        defer_annotations,
    );
    splice_bodies(source, block, [(body_text, body_span)], leaf_edits)
}

/// Returns the leftmost `Name` identifier of an `Attribute` or
/// `Subscript` access chain. `None` when the chain bottoms out at any
/// other expression shape (a parenthesized binary, a literal, a
/// `Call` return).
fn root_name(expr: &Expr) -> Option<&str> {
    let mut current = expr;
    loop {
        match current {
            Expr::Attribute(a) => current = &a.value,
            Expr::Name(n) => return Some(n.id.as_str()),
            Expr::Subscript(s) => current = &s.value,
            _ => return None,
        }
    }
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

/// Assigns each binding a Kahn-style topological tier from its
/// intra-run dependency set. Tier 0 is bindings with no deps, tier N
/// is bindings whose deps all sit in tiers strictly less than N.
/// Returns `None` when any binding remains untiered after `n`
/// iterations.
fn tier_levels(dep_sets: &[HashSet<usize>]) -> Option<Vec<usize>> {
    let n = dep_sets.len();
    let mut tiers: Vec<Option<usize>> = vec![None; n];
    for _ in 0..n {
        for i in 0..n {
            if tiers[i].is_some() || !dep_sets[i].iter().all(|&d| tiers[d].is_some()) {
                continue;
            }
            tiers[i] = Some(
                dep_sets[i]
                    .iter()
                    .filter_map(|&d| tiers[d])
                    .max()
                    .map_or(0, |t| t + 1),
            );
        }
    }
    tiers.into_iter().collect()
}

/// Indexes each name to its position, or `None` when a name repeats. A
/// duplicate makes an intra-run reference ambiguous, so the caller
/// declines the reorder.
fn unique_name_index<'a>(names: impl Iterator<Item = &'a str>) -> Option<HashMap<&'a str, usize>> {
    let mut index = HashMap::new();
    for (position, name) in names.enumerate() {
        if index.insert(name, position).is_some() {
            return None;
        }
    }
    Some(index)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::test_support::parse;

    fn class_member(stmt: &Stmt) -> Option<(&str, &str)> {
        stmt.as_class_def_stmt().map(|class| {
            let name = class.name.as_str();
            (name, name)
        })
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
        let text = crate::primitives::edit::apply_edits(source.text(), edits);
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

    #[test]
    fn assign_run_target_unwraps_both_assign_kinds_and_filters_non_names() {
        let s = parse("X = 1\nself.x = 1\ny: int = 2\nz: int\n(a, b) = (1, 2)\n");
        let targets: Vec<Option<&str>> = s
            .ast()
            .body
            .iter()
            .map(|s| assign_run_target(s).map(|(name, _)| name))
            .collect();
        assert_eq!(targets, vec![Some("X"), None, Some("y"), Some("z"), None]);
    }

    #[test]
    fn collect_leaf_edits_drops_a_keyword_rewrite_overlapping_another_edit() {
        let source = parse(
            "def inner(b, a):\n    pass\n\n\ndef outer(d, c):\n    pass\n\n\nouter(inner(1, 2), 3)\n",
        );
        let edits = collect_leaf_edits(&source, &call_rewrite_targets(&source));
        let text = crate::primitives::edit::apply_edits(source.text(), edits);
        assert_eq!(
            text,
            "def inner(a, b):\n    pass\n\n\ndef outer(c, d):\n    pass\n\n\nouter(c=3, d=inner(1, 2))\n",
        );
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
        let edits = collect_leaf_edits(&source, &call_rewrite_targets(&source));
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

    #[test]
    fn def_run_tier_keys_declines_a_duplicate_member_name() {
        let source = parse("class Dup:\n    pass\n\n\nclass Dup:\n    pass\n");
        assert!(def_run_tier_keys(&source.ast().body, false, class_member).is_none());
    }

    #[test]
    fn def_run_tier_keys_declines_a_reference_cycle() {
        let source = parse("class Alpha(Beta):\n    pass\n\n\nclass Beta(Alpha):\n    pass\n");
        assert!(def_run_tier_keys(&source.ast().body, false, class_member).is_none());
    }

    #[test]
    fn def_run_tier_keys_excludes_a_recursive_self_reference() {
        let source = parse("class Node:\n    def child(self) -> Node: ...\n");
        let body = &source.ast().body;
        let keys =
            def_run_tier_keys(body, false, class_member).expect("self-reference does not decline");
        assert_eq!(keys[&body[0].range().start()].0, 0);
    }

    #[test]
    fn def_run_tier_keys_tiers_a_backward_base_class_reference() {
        let source = parse("class Beta:\n    pass\n\n\nclass Alpha(Beta):\n    pass\n");
        let body = &source.ast().body;
        let keys = def_run_tier_keys(body, false, class_member).expect("acyclic run tiers");
        let tier = |i: usize| keys[&body[i].range().start()].0;
        assert_eq!(tier(0), 0, "Beta has no dependency");
        assert_eq!(tier(1), 1, "Alpha depends on Beta");
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

    #[test]
    fn eval_ref_visitor_collects_eval_surface_and_skips_bodies() {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef:
                    return BodyRef
        "});
        let mut visitor = EvalRefVisitor {
            defer_annotations: false,
            names: Vec::new(),
        };
        visitor.visit_stmt(&source.ast().body[0]);
        let collected: HashSet<&str> = visitor.names.iter().copied().collect();
        assert_eq!(
            collected,
            HashSet::from(["AnnotRef", "BaseRef", "DefaultRef", "ParamRef", "ReturnRef"]),
        );
    }

    #[test]
    fn eval_ref_visitor_prunes_a_lambda_body() {
        let source = parse("class Probe:\n    factory = lambda seed=SeedRef: BodyRef\n");
        let mut visitor = EvalRefVisitor {
            defer_annotations: false,
            names: Vec::new(),
        };
        visitor.visit_stmt(&source.ast().body[0]);
        let collected: HashSet<&str> = visitor.names.iter().copied().collect();
        assert_eq!(collected, HashSet::from(["SeedRef"]));
    }

    #[test]
    fn eval_ref_visitor_skips_annotations_when_deferred() {
        let source = parse(indoc! {"
            class Probe(BaseRef):
                field: AnnotRef

                def method(self, p: ParamRef = DefaultRef) -> ReturnRef: ...
        "});
        let mut visitor = EvalRefVisitor {
            defer_annotations: true,
            names: Vec::new(),
        };
        visitor.visit_stmt(&source.ast().body[0]);
        let collected: HashSet<&str> = visitor.names.iter().copied().collect();
        assert_eq!(collected, HashSet::from(["BaseRef", "DefaultRef"]));
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

    #[rstest]
    #[case("a", Some("a"))]
    #[case("a.b.c", Some("a"))]
    #[case("a[0]", Some("a"))]
    #[case("a.b[c]", Some("a"))]
    #[case("a[b][c]", Some("a"))]
    #[case("(a + b)[c]", None)]
    #[case("1[c]", None)]
    fn root_name_walks_attribute_and_subscript_chains_to_leftmost_identifier(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let s = parse(&format!("x = {src}\n"));
        let assign = s.ast().body[0].as_assign_stmt().expect("assign");
        assert_eq!(root_name(&assign.value), expected);
    }

    #[test]
    fn simple_name_assign_filters_to_single_name_targets() {
        let s = parse("X = 1\nself.x = 1\nx, y = 1, 2\n");
        let names: Vec<Option<&str>> = s.ast().body.iter().map(simple_name_assign).collect();
        assert_eq!(names, vec![Some("X"), None, None]);
    }

    #[test]
    fn tier_levels_assigns_zero_for_empty_deps() {
        let deps = vec![HashSet::new(), HashSet::new(), HashSet::new()];
        assert_eq!(tier_levels(&deps), Some(vec![0, 0, 0]));
    }

    #[test]
    fn tier_levels_climbs_through_chain() {
        let deps = vec![
            HashSet::new(),
            HashSet::from([0]),
            HashSet::from([1]),
            HashSet::from([0, 2]),
        ];
        assert_eq!(tier_levels(&deps), Some(vec![0, 1, 2, 3]));
    }

    #[rstest]
    #[case(vec![HashSet::from([0])])]
    #[case(vec![HashSet::from([1]), HashSet::from([0])])]
    #[case(vec![HashSet::from([1]), HashSet::from([2]), HashSet::from([0])])]
    fn tier_levels_returns_none_on_cycles(#[case] deps: Vec<HashSet<usize>>) {
        assert_eq!(tier_levels(&deps), None);
    }

    proptest! {
        #[test]
        fn tier_levels_assigns_dependency_respecting_tiers_for_dags(
            deps in prop::collection::vec(prop::collection::vec(0usize..16, 0..4), 1..16),
        ) {
            let dag: Vec<HashSet<usize>> = deps
                .into_iter()
                .enumerate()
                .map(|(i, ds)| ds.into_iter().filter(|&d| d < i).collect())
                .collect();
            let Some(tiers) = tier_levels(&dag) else {
                return Err(TestCaseError::fail("acyclic input must tier"));
            };
            for (i, ds) in dag.iter().enumerate() {
                for &d in ds {
                    prop_assert!(
                        tiers[i] > tiers[d],
                        "binding {i} (tier {}) must sit strictly above dep {d} (tier {})",
                        tiers[i],
                        tiers[d],
                    );
                }
            }
        }

        #[test]
        fn tier_levels_rejects_inputs_with_self_loops(
            n in 1usize..8,
            cycle_index in 0usize..8,
        ) {
            let cycle_index = cycle_index.min(n - 1);
            let mut deps: Vec<HashSet<usize>> = (0..n).map(|_| HashSet::new()).collect();
            deps[cycle_index].insert(cycle_index);
            prop_assert_eq!(tier_levels(&deps), None);
        }
    }
}
