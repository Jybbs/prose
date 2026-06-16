//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning. The covered shapes are classes and functions in a body,
//! class-scope `Stmt::AnnAssign` field declarations and `Stmt::Assign`
//! runs with simple `Name` targets, function and lambda keyword-only
//! parameters, call kwargs, set literal elements, consecutive `import`
//! blocks reordered into canonical bare
//! / external-`from` / local-package groups plus their alias lists,
//! `global` and `nonlocal` name lists, `del` target lists, and the
//! string literals inside `__all__` / `__slots__`.
//!
//! Sorting flows through `primitives::orderer::reorder_text`. A
//! recursive `Cow<'src, str>` rewriter folds inner sorts into the
//! outer scope's replacement text, so each outermost reordering scope
//! emits a single edit covering its descendants.
//!
//! Positional-or-keyword parameters never reorder, free function and
//! method alike, because no single-file rewrite can keep every caller's
//! positional binding intact. Only the keyword-only block past `*` sorts.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Decorator, ExceptHandler, ExprDict, PythonVersion, Stmt, StmtAnnAssign, StmtFunctionDef,
    helpers::{any_over_expr, is_compound_statement, is_dunder, map_callable},
};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        binding::{annotated_name_target, single_name_target, tail_identifier},
        comments::{is_banner_block, leading_comment_block, marker_floor},
        edit::{apply_inline_edits, narrowed_replacement, singleton_groups},
        imports::{future_annotations_alias, import_sort_key, same_import_group},
        orderer::{assemble_blocks, block_range, blocks_span, permute_in_place},
        params::pins_positional_params,
        scope::{BodyScope, scoped_body},
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod bands;
mod class_graph;
mod dict;
mod leaves;
mod tiering;

use self::{
    bands::{Banding, band_module_constants, banded_gap},
    class_graph::permute_class_assigns,
    leaves::{collect_docstring_entry_edits, collect_leaf_edits},
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
        let (mut leaf_edits, param_docs) = collect_leaf_edits(source);
        if self.docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source, &param_docs));
            leaf_edits.sort_unstable();
        }
        let ctx = RewriteCtx {
            defer_annotations: defers_annotations(body),
            first_party: &self.first_party,
            leaf_edits: &leaf_edits,
            source,
            target_version: self.target_version,
        };
        let (body_text, body_span) =
            rewrite_body(ctx, body, source.module_range(), BodyScope::Module);
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
    Some((ann, annotated_name_target(ann)?))
}

/// Returns the slot ranges of consecutive items whose pairwise
/// neighbors satisfy `adjacent`. Singleton runs drop.
fn chunk_runs(items: &[Stmt], mut adjacent: impl FnMut(&Stmt, &Stmt) -> bool) -> Vec<Range<usize>> {
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
    tail_identifier(map_callable(&decorator.expression))
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
        .any(|pair| source.same_line(pair[0].end(), pair[1].start()))
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

/// True when a recognized section marker, a banner or hash heading, sits
/// in the gap between two member blocks, opening a section the sort never
/// reorders across.
fn marker_in_gap(source: &Source, lower: TextSize, upper: TextSize) -> bool {
    leading_comment_block(source, lower, upper).is_some_and(|block| is_banner_block(source, block))
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
        ..
    } = ctx;
    let (mut blocks, mut rendered): (Vec<TextRange>, Vec<Cow<'a, str>>) = body
        .iter()
        .enumerate()
        .map(|(i, stmt)| {
            let raw = block_range(source, body, i, outer);
            let block = TextRange::new(marker_floor(source, raw.start(), stmt.start()), raw.end());
            (block, rewrite_stmt(ctx, stmt, block, scope))
        })
        .unzip();
    let body_span = blocks_span(&blocks);
    let n = body.len();
    let mut order: Vec<usize> = (0..n).collect();
    let mut import_run_slots: Vec<usize> = Vec::new();
    let mut band: Option<Banding> = None;
    if !has_inline_statement_join(source, body) {
        let sections = section_ranges(source, &blocks);
        let boundaries: Vec<usize> = sections.iter().skip(1).map(|s| s.start).collect();
        let in_class = scope == BodyScope::Class;
        let is_import = |s: &Stmt| s.is_import_stmt() || s.is_import_from_stmt();
        for section in &sections {
            let members = &body[section.clone()];
            if scope != BodyScope::Function {
                permute_defs(&mut order, body, section.clone(), defer_annotations, |s| {
                    s.as_class_def_stmt().map(|c| {
                        let name = c.name.as_str();
                        (name, name)
                    })
                });
                if in_class {
                    permute_class_assigns(&mut order, body, section.clone(), defer_annotations);
                }
                if !(in_class && class_pins_methods(members)) {
                    permute_defs(&mut order, body, section.clone(), defer_annotations, |s| {
                        s.as_function_def_stmt().map(|f| {
                            let name = f.name.as_str();
                            (name, (method_group(f), name))
                        })
                    });
                }
            }
            for Range { start, end } in chunk_runs(members, |a, b| is_import(a) && is_import(b)) {
                permute_in_place(
                    &mut order,
                    body,
                    section.start + start..section.start + end,
                    |s| import_sort_key(s, first_party),
                );
            }
        }
        if scope == BodyScope::Module {
            band = band_module_constants(ctx, body, &blocks, &boundaries, &mut order);
            // A banded constant carries its forward-attached prose comment
            // up with it: its block extends back over the comment and its
            // rendered text gains the comment, collapsing the gap below so
            // the hoist relocates the comment rather than stranding it.
            for &(idx, comment) in band.iter().flat_map(|b| &b.carries) {
                let carried = format!(
                    "{}{}{}",
                    source.slice(comment),
                    source.newline_str(),
                    rendered[idx],
                );
                blocks[idx] = comment.cover(blocks[idx]);
                rendered[idx] = Cow::Owned(carried);
            }
        }
        // A banded order reconstructs its own blank-line texture. Otherwise
        // same-group import neighbors collapse to one line, except across a
        // section marker, whose dividing gap must survive in place.
        if band.is_none() {
            import_run_slots.extend((0..n.saturating_sub(1)).filter(|&slot| {
                same_import_group(&body[order[slot]], &body[order[slot + 1]], first_party)
                    && boundaries.binary_search(&(slot + 1)).is_err()
            }));
        }
    }
    let any_owned = rendered.iter().any(|c| matches!(c, Cow::Owned(_)));
    let identity = order.iter().copied().eq(0..n);
    if !any_owned && identity && import_run_slots.is_empty() {
        return (Cow::Borrowed(source.slice(body_span)), body_span);
    }
    let assembled = assemble_blocks(source, &blocks, &rendered, &order, |i| match &band {
        Some(b) => banded_gap(&b.ranks, body, first_party, order[i], order[i + 1]),
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
    let Some((body, scope)) = scoped_body(stmt) else {
        if is_compound_statement(stmt) {
            return rewrite_compound(ctx, stmt, block, parent_scope);
        }
        return apply_inline_edits(ctx.source, block, ctx.leaf_edits);
    };
    if body.is_empty() {
        return apply_inline_edits(ctx.source, block, ctx.leaf_edits);
    }
    let (body_text, body_span) = rewrite_body(ctx, body, stmt.range(), scope);
    splice_bodies(ctx.source, block, [(body_text, body_span)], ctx.leaf_edits)
}

/// Splits the body's slots into sections at each gap carrying a section
/// marker, so a family sort orders members within a section and never
/// moves one across the marker dividing it from its neighbor.
fn section_ranges(source: &Source, blocks: &[TextRange]) -> Vec<Range<usize>> {
    let mut sections = Vec::new();
    let mut start = 0;
    for i in 1..blocks.len() {
        if marker_in_gap(source, blocks[i - 1].end(), blocks[i].start()) {
            sections.push(start..i);
            start = i;
        }
    }
    sections.push(start..blocks.len());
    sections
}

/// Returns the simple name assigned by an `Stmt::Assign` whose
/// target is a single `Name`. `None` for multi-target,
/// destructuring, attribute, or subscript targets.
fn simple_name_assign(stmt: &Stmt) -> Option<&str> {
    single_name_target(stmt.as_assign_stmt()?)
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

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, first_class, first_def, parse};

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
        let f = first_def(&s);
        let decorator = f.decorator_list.first().expect("one decorator");
        assert_eq!(decorator_simple_name(decorator), expected);
    }

    #[test]
    fn decorator_simple_name_returns_none_for_complex_expressions() {
        let s = parse("@(some_factory())()\ndef f(): pass\n");
        let f = first_def(&s);
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
        let class = first_class(&s);
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
