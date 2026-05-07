//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning. The covered shapes are classes and functions in a body,
//! class-scope `Stmt::AnnAssign` field declarations and `Stmt::Assign`
//! runs with simple `Name` targets, function and lambda parameters
//! with `self` / `cls` and decorators carrying positional arguments
//! pinned, call kwargs, set literal elements, `from` and bare
//! `import` runs plus their alias lists, `global` and `nonlocal`
//! name lists, `del` target lists, and the string literals inside
//! `__all__` / `__slots__`.
//!
//! Sorting flows through `primitives::orderer::reorder_text`. A
//! recursive `Cow<'src, str>` rewriter folds inner sorts into the
//! outer scope's replacement text, so each outermost reordering scope
//! emits a single edit covering its descendants.

use std::borrow::Cow;
use std::ops::Range;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::{any_over_expr, is_compound_statement, is_dunder, map_callable};
use ruff_python_ast::visitor::{walk_expr, walk_stmt, Visitor as AstVisitor};
use ruff_python_ast::{
    Alias, Decorator, DictItem, ExceptHandler, Expr, ExprCall, ExprDict, ExprLambda, ExprSet,
    Identifier, ParameterWithDefault, Parameters, Stmt, StmtAnnAssign, StmtAssign, StmtDelete,
    StmtFunctionDef,
};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::config::Config;
use crate::primitives::edit::{apply_inline_edits, narrowed_replacement};
use crate::primitives::orderer::{
    assemble_blocks, block_range, blocks_span, permute_full, permute_in_place, reorder_text,
};
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct Alphabetize;

impl Alphabetize {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for Alphabetize {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let leaf_edits = collect_leaf_edits(source);
        let (body_text, body_span) = rewrite_body(
            source,
            body,
            TextRange::up_to(source.text().text_len()),
            BodyScope::Module,
            &leaf_edits,
        );
        match body_text {
            Cow::Borrowed(_) => leaf_edits,
            Cow::Owned(text) => narrowed_replacement(source, body_span, text)
                .into_iter()
                .collect(),
        }
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(Alphabetize))
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum BodyScope {
    Class,
    Function,
    Module,
}

struct LeafCollector<'a> {
    dict_depth: u32,
    edits: Vec<Edit>,
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
        if !pin_positional {
            self.try_emit_inline_reorder(&params.posonlyargs, classify_param);
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

/// Returns the `StmtAnnAssign` and its target name when the target
/// is a single `Name`.
fn ann_assign_with_named_field(stmt: &Stmt) -> Option<(&StmtAnnAssign, &str)> {
    let ann = stmt.as_ann_assign_stmt()?;
    Some((ann, ann.target.as_name_expr()?.id.as_str()))
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

/// Walks the AST collecting every leaf-level sort edit. Each emitted
/// edit covers a narrow range inside a single `Stmt` or `Expr`, so
/// the resulting edits are non-overlapping with each other.
fn collect_leaf_edits(source: &Source) -> Vec<Edit> {
    let mut collector = LeafCollector {
        dict_depth: 0,
        edits: Vec::new(),
        source,
    };
    collector.visit_body(&source.ast().body);
    collector.edits
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

/// True when any of `f`'s decorators is a `Call` carrying positional
/// arguments, signalling the decorator may bind values into the
/// signature by position.
fn pins_positional_params(f: &StmtFunctionDef) -> bool {
    f.decorator_list.iter().any(|d| {
        d.expression
            .as_call_expr()
            .is_some_and(|c| !c.arguments.args.is_empty())
    })
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
) -> (Cow<'src, str>, TextRange) {
    let (blocks, rendered): (Vec<TextRange>, Vec<Cow<'src, str>>) = body
        .iter()
        .enumerate()
        .map(|(i, stmt)| {
            let block = block_range(source, body, i, outer);
            (block, rewrite_stmt(source, stmt, block, scope, leaf_edits))
        })
        .unzip();
    let body_span = blocks_span(&blocks);
    let n = body.len();
    let mut order: Vec<usize> = (0..n).collect();
    let in_class = scope == BodyScope::Class;
    if scope != BodyScope::Function {
        permute_full(&mut order, body, |s| {
            s.as_class_def_stmt().map(|c| c.name.as_str())
        });
        if in_class {
            permute_full(&mut order, body, |s| {
                ann_assign_with_named_field(s).map(|(ann, name)| (u8::from(has_default(ann)), name))
            });
            permute_full(&mut order, body, simple_name_assign);
        }
        if !(in_class && class_pins_methods(body)) {
            permute_full(&mut order, body, |s| {
                s.as_function_def_stmt()
                    .map(|f| (method_group(f), f.name.as_str()))
            });
        }
    }
    let mut import_run_slots: Vec<usize> = Vec::new();
    for Range { start, end } in statement_run_ranges(body, Stmt::is_import_from_stmt) {
        permute_in_place(&mut order, body, start..end, |s| {
            let i = s.as_import_from_stmt()?;
            Some((i.level, i.module.as_deref().unwrap_or_default()))
        });
        import_run_slots.extend(start..end - 1);
    }
    for Range { start, end } in statement_run_ranges(body, Stmt::is_import_stmt) {
        permute_in_place(&mut order, body, start..end, |s| {
            s.as_import_stmt()?
                .names
                .iter()
                .map(|a| a.name.as_str())
                .min()
        });
        import_run_slots.extend(start..end - 1);
    }
    import_run_slots.sort_unstable();
    let any_owned = rendered.iter().any(|c| matches!(c, Cow::Owned(_)));
    let identity = order.iter().copied().eq(0..n);
    if !any_owned && identity && import_run_slots.is_empty() {
        return (Cow::Borrowed(source.slice(body_span)), body_span);
    }
    let assembled = assemble_blocks(source, &blocks, &rendered, &order, |i| {
        import_run_slots.binary_search(&i).ok().map(|_| "\n")
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
) -> Cow<'src, str> {
    let bodies = compound_sub_bodies(stmt)
        .into_iter()
        .filter(|(body, _)| !body.is_empty())
        .map(|(body, outer)| rewrite_body(source, body, outer, scope, leaf_edits));
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
) -> Cow<'src, str> {
    let (body, body_outer, scope): (&[Stmt], TextRange, BodyScope) = match stmt {
        Stmt::ClassDef(c) => (&c.body, c.range(), BodyScope::Class),
        Stmt::FunctionDef(f) => (&f.body, f.range(), BodyScope::Function),
        s if is_compound_statement(s) => {
            return rewrite_compound(source, stmt, block, parent_scope, leaf_edits)
        }
        _ => return apply_inline_edits(source, block, leaf_edits),
    };
    if body.is_empty() {
        return apply_inline_edits(source, block, leaf_edits);
    }
    let (body_text, body_span) = rewrite_body(source, body, body_outer, scope, leaf_edits);
    splice_bodies(source, block, [(body_text, body_span)], leaf_edits)
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
/// match `predicate`. Adjacent matching statements stay in the same
/// run regardless of any blank lines between them, so all imports
/// of the same form collapse into one block at the top of the body.
/// Non-matching statements break the run. Singleton runs drop.
fn statement_run_ranges(
    body: &[Stmt],
    mut predicate: impl FnMut(&Stmt) -> bool,
) -> Vec<Range<usize>> {
    let mut start = 0;
    body.chunk_by(|a, b| predicate(a) && predicate(b))
        .filter_map(|chunk| {
            let end = start + chunk.len();
            let range = (chunk.len() >= 2).then_some(start..end);
            start = end;
            range
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::test_support::parse;

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
    fn collect_leaf_edits_yields_edits_in_source_order() {
        let src = indoc! {"
            import b, a
            from m import d, c
            __all__ = ['z', 'y']
            x = {z, y}
            def f(b, a): foo(b=2, a=1)
        "};
        let edits = collect_leaf_edits(&parse(src));
        assert!(edits.len() >= 5, "fixture must trigger multiple producers");
        assert!(
            edits.is_sorted(),
            "leaf edits must be emitted in source order; partition_point in apply_inline_edits relies on this",
        );
    }

    #[test]
    fn decorator_simple_name_extracts_rightmost_segment() {
        for (src, expected) in [
            ("@property\ndef f(): pass\n", Some("property")),
            (
                "@functools.cached_property\ndef f(): pass\n",
                Some("cached_property"),
            ),
            ("@click.option(\"--name\")\ndef f(): pass\n", Some("option")),
            (
                "@pytest.mark.parametrize(\"a\", [1])\ndef f(): pass\n",
                Some("parametrize"),
            ),
            ("@functools.wraps(other)\ndef f(): pass\n", Some("wraps")),
        ] {
            let s = parse(src);
            let f = s.ast().body[0].as_function_def_stmt().expect("def");
            let decorator = f.decorator_list.first().expect("one decorator");
            assert_eq!(decorator_simple_name(decorator), expected, "src = {src}");
        }
    }

    #[test]
    fn decorator_simple_name_returns_none_for_complex_expressions() {
        let s = parse("@(some_factory())()\ndef f(): pass\n");
        let f = s.ast().body[0].as_function_def_stmt().expect("def");
        let decorator = f.decorator_list.first().expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), None);
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
