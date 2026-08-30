//! The body-rewrite recursion `alphabetize-siblings` drives: the
//! per-body layout each scope resolves, the recursion splicing a
//! rewritten body back into its parent, and the divider an import-run
//! collapse seats.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, helpers::is_compound_statement};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    class_graph::permute_class_assigns,
    members::{class_pins_methods, function_key},
    module_graph::permute_module_defs,
};
use crate::{
    primitives::{
        constructor::keyword_field_start,
        decorator::is_decorated,
        edit::{apply_inline_edits, splice_bodies},
        imports::{import_blank_lines, import_sort_key, sectioned_import_runs},
        orderer::{
            adjacent_slots, any_sibling_shares_line, assemble_or_borrow, permute_runs,
            rendered_member_blocks,
        },
        scope::{BodyScope, scoped_body, splice_compound_arms},
        sections::Sections,
        tiering::{CallReach, Evaluated, call_reachable, calls_a_name, permute_defs},
    },
    source::Source,
};

/// The reorder layout of one body: its member blocks, their rendered
/// text, the new-order permutation, and the new-order slots whose import
/// neighbor collapses onto one line. [`rewrite_body`] folds it into the
/// combined `Cow` and the notebook path splits it per cell.
pub(super) struct BodyLayout<'a> {
    pub(super) blocks: Vec<TextRange>,
    pub(super) import_run_slots: Vec<usize>,
    pub(super) order: Vec<usize>,
    pub(super) rendered: Vec<Cow<'a, str>>,
}

/// Context threaded through the body-rewrite recursion, every field
/// invariant but `keyword_fields_from`, which each class header refreshes
/// for its own body.
#[derive(Clone, Copy)]
pub(super) struct RewriteCtx<'a> {
    pub(super) defer_annotations: bool,
    pub(super) first_party: &'a [String],
    pub(super) group_imports: bool,
    pub(super) group_methods: bool,
    pub(super) keyword_fields_from: TextSize,
    pub(super) leaf_edits: &'a [Edit],
    pub(super) sort_definitions: bool,
    pub(super) source: &'a Source,
}

/// Computes the reorder of `body`: renders each member, then permutes the
/// slots within each section by the family sorts and import grouping that
/// `scope` enables, leaving the assembly to the caller. The section
/// partition walls each notebook cell, so no permutation crosses a cell.
pub(super) fn body_layout<'a>(
    ctx: RewriteCtx<'a>,
    body: &'a [Stmt],
    outer: TextRange,
    scope: BodyScope,
) -> BodyLayout<'a> {
    let RewriteCtx {
        defer_annotations,
        first_party,
        group_imports,
        group_methods,
        keyword_fields_from,
        sort_definitions,
        source,
        ..
    } = ctx;
    let (blocks, rendered) = rendered_member_blocks(source, body, outer, |stmt, block| {
        rewrite_stmt(ctx, stmt, block, scope)
    });
    let mut order: Vec<usize> = (0..body.len()).collect();
    let mut import_run_slots: Vec<usize> = Vec::new();
    if !any_sibling_shares_line(source, body) {
        let sections = Sections::of(source, &blocks);
        let in_class = scope == BodyScope::Class;
        if scope != BodyScope::Function {
            let holds = |stmt: &Stmt| !in_class && is_decorated(stmt);
            // Only a non-definition statement consults the call graph,
            // so a body holding definitions alone builds none.
            let consults_calls = body.iter().any(|stmt| {
                !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) && calls_a_name(stmt)
            });
            let reachable = if consults_calls {
                call_reachable(source.binding_analysis(), body)
            } else {
                CallReach::default()
            };
            let evaluated = Evaluated::of(body, &reachable, defer_annotations);
            let evaluation = evaluated.evaluation();
            // A permutation reverted for a reference that a later
            // permutation relocates becomes legal once that one lands, so
            // the section's permutations run to a fixed point rather than
            // leaving the rest of the sort to a second pass.
            for _ in 0..body.len().max(1) {
                let settled = order.clone();
                for section in sections.ranges() {
                    // A class body keeps a pass per family, the method sort
                    // carrying its own pinned-field gate, whereas module
                    // scope sorts its classes and functions as one banded
                    // run.
                    if in_class {
                        if sort_definitions {
                            permute_defs(
                                &mut order,
                                body,
                                section.clone(),
                                evaluation,
                                holds,
                                |s| {
                                    s.as_class_def_stmt().map(|c| {
                                        let name = c.name.as_str();
                                        (name, name)
                                    })
                                },
                                |tier, key| (tier, key),
                            );
                        }
                        permute_class_assigns(
                            &mut order,
                            body,
                            section.clone(),
                            evaluation,
                            keyword_fields_from,
                        );
                        if sort_definitions && !class_pins_methods(&body[section.clone()]) {
                            permute_defs(
                                &mut order,
                                body,
                                section.clone(),
                                evaluation,
                                holds,
                                |s| {
                                    s.as_function_def_stmt()
                                        .map(|f| (f.name.as_str(), function_key(f, group_methods)))
                                },
                                |tier, key| (tier, key),
                            );
                        }
                    } else if sort_definitions {
                        permute_module_defs(
                            &mut order,
                            body,
                            section.clone(),
                            evaluation,
                            holds,
                            group_methods,
                        );
                    }
                }
                if order == settled {
                    break;
                }
            }
        }
        permute_runs(
            &mut order,
            body,
            sectioned_import_runs(&sections, body),
            |s| import_sort_key(s, first_party, group_imports),
        );
        // Same-group import neighbors collapse to one line, except across a
        // section marker, whose dividing gap must survive in place. A slot
        // gap holding a comment and a member block opening on a bound run
        // both keep their source gap, so no collapse deletes or reseats a
        // comment.
        import_run_slots = adjacent_slots(&order, |slot, a, b| {
            import_blank_lines(&body[a], &body[b], first_party, group_imports) == Some(0)
                && !sections.is_boundary(slot + 1)
                && source
                    .comment_ranges()
                    .comments_in_range(TextRange::new(blocks[slot].end(), blocks[slot + 1].start()))
                    .is_empty()
                && blocks[b].start() == source.text().line_start(body[b].start())
        });
    }
    BodyLayout {
        blocks,
        import_run_slots,
        order,
        rendered,
    }
}

/// The one-newline divider an import-run collapse inserts after new-order
/// slot `i`, written in the ending `source` carries. `None` where the
/// neighbors do not collapse onto one line.
pub(super) fn import_gap(
    source: &Source,
    import_run_slots: &[usize],
    i: usize,
) -> Option<&'static str> {
    import_run_slots
        .binary_search(&i)
        .is_ok()
        .then_some(source.newline_str())
}

/// Rewrites a non-empty body, returning the rewritten text alongside
/// the block-extent span it covers. The text is `Cow::Owned` when any
/// sibling reorder fires, any descendant rewrite produces owned
/// content, or any leaf edit lands inside, falling back to
/// `Cow::Borrowed` over `source.slice(span)`. `scope` selects which
/// family sorts apply.
fn rewrite_body<'a>(
    ctx: RewriteCtx<'a>,
    body: &'a [Stmt],
    outer: TextRange,
    scope: BodyScope,
) -> (Cow<'a, str>, TextRange) {
    let layout = body_layout(ctx, body, outer, scope);
    assemble_or_borrow(
        ctx.source,
        &layout.blocks,
        &layout.rendered,
        &layout.order,
        !layout.import_run_slots.is_empty(),
        |i| import_gap(ctx.source, &layout.import_run_slots, i),
    )
}

/// Recurses into each sub-body of a compound statement, splicing
/// rewritten bodies back into the parent block while leaving header,
/// keyword, and inter-arm regions to leaf-level edits.
fn rewrite_compound<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &'a Stmt,
    block: TextRange,
    scope: BodyScope,
) -> Cow<'a, str> {
    splice_compound_arms(ctx.source, stmt, block, ctx.leaf_edits, |body, outer| {
        rewrite_body(ctx, body, outer, scope)
    })
}

/// Rewrites a single statement. Classes and functions fold their body
/// via `rewrite_body` and splice the result. Compound statements
/// (`if`, `for`, `while`, `with`, `try`, `match`) recurse into each
/// sub-body with the inherited `parent_scope`, so module-level reorders
/// (imports, classes, top-level functions) fire inside `if TYPE_CHECKING`
/// and other body-bearing arms. Other shapes apply leaf edits in place.
fn rewrite_stmt<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &'a Stmt,
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
    let ctx = stmt.as_class_def_stmt().map_or(ctx, |class| RewriteCtx {
        keyword_fields_from: keyword_field_start(class),
        ..ctx
    });
    let (body_text, body_span) = rewrite_body(ctx, body, stmt.range(), scope);
    splice_bodies(ctx.source, block, [(body_text, body_span)], ctx.leaf_edits)
}
