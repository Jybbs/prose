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
use ruff_python_ast::helpers::{any_over_expr, is_dunder, map_callable};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{walk_expr, walk_stmt, Visitor as AstVisitor};
use ruff_python_ast::{
    Decorator, Expr, ExprCall, ExprLambda, ExprSet, ParameterWithDefault, Parameters, Stmt,
    StmtAnnAssign, StmtAssign, StmtDelete, StmtFunctionDef,
};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::edit::{apply_inline_edits, narrow_edit};
use crate::primitives::orderer::{assemble_blocks, block_range, permute_in_place, reorder_text};
use crate::source::Source;

pub(crate) struct Alphabetize;

impl Alphabetize {
    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum BodyScope {
    Class,
    Function,
    Module,
}

impl Rule for Alphabetize {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let leaf_edits = collect_leaf_edits(source);
        let outer = TextRange::up_to(source.text().text_len());
        let (body_text, body_span) =
            rewrite_body(source, body, outer, BodyScope::Module, &leaf_edits);
        match body_text {
            Cow::Borrowed(_) => leaf_edits,
            Cow::Owned(text) => emit_narrowed(source, body_span, text),
        }
    }

    fn name(&self) -> &'static str {
        "alphabetize"
    }
}

fn emit_narrowed(source: &Source, span: TextRange, text: String) -> Vec<Edit> {
    let Some((narrowed_span, narrowed_text)) = narrow_edit(text, span, source.slice(span)) else {
        return Vec::new();
    };
    vec![if narrowed_text.is_empty() {
        Edit::range_deletion(narrowed_span)
    } else {
        Edit::range_replacement(narrowed_text, narrowed_span)
    }]
}

/// Rewrites a non-empty body, returning the rewritten text alongside
/// the block-extent span it covers. The text is `Cow::Owned` when any
/// sibling reorder fires, any descendant rewrite produces owned
/// content, or any leaf edit lands inside; otherwise `Cow::Borrowed`
/// over `source.slice(span)`. `scope` selects which family sorts apply.
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
            (block, rewrite_stmt(source, stmt, block, leaf_edits))
        })
        .unzip();
    let blocks_span = blocks[0].cover(*blocks.last().expect("non-empty"));
    let mut order: Vec<usize> = (0..body.len()).collect();
    let in_class = scope == BodyScope::Class;
    if scope != BodyScope::Function {
        permute_in_place(&mut order, body, 0..body.len(), |s| {
            s.as_class_def_stmt().map(|c| c.name.as_str())
        });
        if in_class {
            permute_in_place(&mut order, body, 0..body.len(), |s| {
                ann_assign_with_named_field(s).map(|(ann, name)| (u8::from(has_default(ann)), name))
            });
            permute_in_place(&mut order, body, 0..body.len(), simple_name_assign);
        }
        if !(in_class && class_pins_methods(body)) {
            permute_in_place(&mut order, body, 0..body.len(), |s| {
                s.as_function_def_stmt()
                    .map(|f| (method_group(f), f.name.as_str()))
            });
        }
        for run in statement_run_ranges(source, body, Stmt::is_import_from_stmt) {
            permute_in_place(&mut order, body, run, |s| {
                let i = s.as_import_from_stmt()?;
                Some((i.level, i.module.as_deref().unwrap_or_default()))
            });
        }
        for run in statement_run_ranges(source, body, Stmt::is_import_stmt) {
            permute_in_place(&mut order, body, run, |s| {
                Some(s.as_import_stmt()?.names.first()?.name.as_str())
            });
        }
    }
    let any_owned = rendered.iter().any(|c| matches!(c, Cow::Owned(_)));
    let identity = order.iter().copied().eq(0..body.len());
    if !any_owned && identity {
        return (Cow::Borrowed(source.slice(blocks_span)), blocks_span);
    }
    (
        Cow::Owned(assemble_blocks(source, &blocks, &rendered, &order)),
        blocks_span,
    )
}

/// Rewrites a single statement. Classes and functions fold their body
/// via `rewrite_body` and splice the result; other shapes apply leaf
/// edits in place.
fn rewrite_stmt<'src>(
    source: &'src Source,
    stmt: &Stmt,
    block: TextRange,
    leaf_edits: &[Edit],
) -> Cow<'src, str> {
    let (body, body_outer, scope): (&[Stmt], TextRange, BodyScope) = match stmt {
        Stmt::ClassDef(c) => (&c.body, c.range(), BodyScope::Class),
        Stmt::FunctionDef(f) => (&f.body, f.range(), BodyScope::Function),
        _ => return apply_inline_edits(source, block, leaf_edits),
    };
    if body.is_empty() {
        return apply_inline_edits(source, block, leaf_edits);
    }
    let (body_text, body_span) = rewrite_body(source, body, body_outer, scope, leaf_edits);
    splice_body(source, block, body_span, body_text, leaf_edits)
}

/// Splices a rewritten body back into its enclosing block, folding
/// leaf edits in the pre-body header and post-body trailer.
fn splice_body<'src>(
    source: &'src Source,
    block: TextRange,
    body_span: TextRange,
    body_text: Cow<'src, str>,
    leaf_edits: &[Edit],
) -> Cow<'src, str> {
    let parts = [
        apply_inline_edits(
            source,
            TextRange::new(block.start(), body_span.start()),
            leaf_edits,
        ),
        body_text,
        apply_inline_edits(
            source,
            TextRange::new(body_span.end(), block.end()),
            leaf_edits,
        ),
    ];
    if parts.iter().all(|p| matches!(p, Cow::Borrowed(_))) {
        return Cow::Borrowed(source.slice(block));
    }
    Cow::Owned(parts.concat())
}

/// Builds a `Vec<Range<usize>>` of contiguous body slots whose
/// statements all match `predicate` and are line-adjacent to their
/// neighbors. Singleton runs drop.
fn statement_run_ranges(
    source: &Source,
    body: &[Stmt],
    mut predicate: impl FnMut(&Stmt) -> bool,
) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut start = 0;
    for chunk in body.chunk_by(|a, b| {
        predicate(a) && predicate(b) && source.is_line_adjacent(TextRange::new(a.end(), b.start()))
    }) {
        let end = start + chunk.len();
        if chunk.len() >= 2 {
            runs.push(start..end);
        }
        start = end;
    }
    runs
}

/// Walks the AST collecting every leaf-level sort edit. Each emitted
/// edit covers a narrow range inside a single `Stmt` or `Expr`, so
/// the resulting edits are non-overlapping with each other.
fn collect_leaf_edits(source: &Source) -> Vec<Edit> {
    let mut collector = LeafCollector {
        edits: Vec::new(),
        source,
    };
    collector.visit_body(&source.ast().body);
    collector.edits
}

struct LeafCollector<'a> {
    edits: Vec<Edit>,
    source: &'a Source,
}

impl<'a> LeafCollector<'a> {
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
        let cow = reorder_text(self.source, items, classify, |_, slice| {
            Cow::Borrowed(slice)
        });
        if let Cow::Owned(text) = cow {
            self.edits.push(Edit::range_replacement(text, span));
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

    fn emit_parameters(&mut self, params: &'a Parameters, pin_positional: bool) {
        if !pin_positional {
            self.try_emit_inline_reorder(&params.posonlyargs, classify_param);
            self.try_emit_inline_reorder(&params.args, classify_param);
        }
        self.try_emit_inline_reorder(&params.kwonlyargs, classify_param);
    }

    fn emit_call(&mut self, c: &'a ExprCall) {
        self.try_emit_inline_reorder(&c.arguments.keywords, |kw| kw.arg.as_deref());
    }

    fn emit_lambda(&mut self, l: &'a ExprLambda) {
        if let Some(params) = l.parameters.as_deref() {
            self.emit_parameters(params, false);
        }
    }

    fn emit_set(&mut self, s: &'a ExprSet) {
        self.try_emit_inline_reorder(&s.elts, |e| {
            (!e.is_starred_expr()).then(|| self.source.slice(e))
        });
    }

    fn emit_delete(&mut self, d: &'a StmtDelete) {
        self.try_emit_inline_reorder(&d.targets, |t| Some(self.source.slice(t)));
    }
}

impl<'a> AstVisitor<'a> for LeafCollector<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
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
            Stmt::Global(g) => {
                self.try_emit_inline_reorder(&g.names, |id| Some(id.as_str()));
            }
            Stmt::Import(i) => {
                self.try_emit_inline_reorder(&i.names, |a| Some(a.name.as_str()));
            }
            Stmt::ImportFrom(i) => {
                self.try_emit_inline_reorder(&i.names, |a| Some(a.name.as_str()));
            }
            Stmt::Nonlocal(n) => {
                self.try_emit_inline_reorder(&n.names, |id| Some(id.as_str()));
            }
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

/// True when a class body has at least two `Stmt::AnnAssign` field
/// declarations and at least one method whose decorator carries
/// positional arguments.
fn class_pins_methods(body: &[Stmt]) -> bool {
    let mut fields = body
        .iter()
        .filter(|s| ann_assign_with_named_field(s).is_some());
    if fields.nth(1).is_none() {
        return false;
    }
    body.iter()
        .filter_map(Stmt::as_function_def_stmt)
        .any(pins_positional_params)
}

/// Composite parameter sort key. Required parameters (no default)
/// sort before optional parameters (has default), each sub-group by
/// name. `self` and `cls` pin in place.
fn classify_param(p: &ParameterWithDefault) -> Option<(u8, &str)> {
    let name = p.name().as_str();
    if matches!(name, "self" | "cls") {
        return None;
    }
    Some((u8::from(p.default.is_some()), name))
}

fn decorator_simple_name(decorator: &Decorator) -> Option<&str> {
    UnqualifiedName::from_expr(map_callable(&decorator.expression))?
        .segments()
        .last()
        .copied()
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

/// Returns the method-group index. `0` for dunders, `1` for
/// `@property` / `@cached_property` (decided by the first decorator),
/// `2` for single-leading-underscore privates, `3` for public.
fn method_group(f: &StmtFunctionDef) -> u8 {
    let name = f.name.as_str();
    if is_dunder(name) {
        return 0;
    }
    let is_property = f
        .decorator_list
        .first()
        .and_then(decorator_simple_name)
        .is_some_and(|n| matches!(n, "property" | "cached_property"));
    if is_property {
        return 1;
    }
    if name.starts_with('_') {
        return 2;
    }
    3
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn parse(src: &str) -> Source {
        Source::from_str(src).expect("test source parses")
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
    fn simple_name_assign_filters_to_single_name_targets() {
        let s = parse("X = 1\nself.x = 1\nx, y = 1, 2\n");
        let names: Vec<Option<&str>> = s.ast().body.iter().map(simple_name_assign).collect();
        assert_eq!(names, vec![Some("X"), None, None]);
    }
}
