//! The body-rewrite recursion `alphabetize-siblings` drives: the
//! per-body layout each scope resolves, the recursion splicing a
//! rewritten body back into its parent, the edits it makes inside a
//! statement a suppression pins, and the divider an import-run collapse
//! seats, beside the rows a class body's sort seats, which a rule
//! forecasting the sort reads.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, helpers::is_compound_statement};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    AlphabetizeSiblings,
    enums::{Enumerations, class_orders_members},
    section_runs::{SectionRuns, fenced_runs},
};
use crate::{
    primitives::{
        constructor::keyword_field_start,
        decorator::is_decorated,
        edit::{apply_inline_edits, narrowed_replacement, splice_bodies},
        imports::{import_blank_lines, import_sort_key, sectioned_import_runs},
        orderer::{
            Assembly, adjacent_slots, any_sibling_shares_line, member_blocks, permute_runs,
            rendered_member_blocks, seated_rows,
        },
        range::blocks_span,
        scope::{BodyScope, scoped_body, splice_compound_arms, sub_bodies, sub_bodies_keep_order},
        sections::Sections,
        tiering::{
            CallReach, Evaluated, call_reachable, consults_call_graph, definition_names,
            eval_time_refs_of, fenced_slots,
        },
    },
    source::Source,
};

/// The reorder layout of one body, its assembly beside the edits made
/// inside each member a suppression pins and the new-order slots whose
/// import neighbor collapses onto one line. [`rewrite_body`] folds it
/// into the combined `Cow` and the notebook path splits it per cell.
pub(super) struct BodyLayout<'a> {
    pub(super) assembly: Assembly<'a>,
    pub(super) held: Vec<Edit>,
    pub(super) import_run_slots: Vec<usize>,
}

/// Context threaded through the body-rewrite recursion, every field
/// invariant but `keeps_order`, `keyword_fields_from`, and
/// `orders_members`, which `rewrite_stmt` refreshes at each class
/// header, `keeps_order` also clearing at each function header, for the
/// body it opens and for every arm nested inside it.
#[derive(Clone, Copy)]
pub(super) struct RewriteCtx<'a> {
    pub(super) defer_annotations: bool,
    pub(super) enumerations: &'a Enumerations<'a>,
    pub(super) first_party: &'a [String],
    pub(super) group_imports: bool,
    pub(super) group_methods: bool,
    pub(super) keeps_order: bool,
    pub(super) keyword_fields_from: TextSize,
    pub(super) leaf_edits: &'a [Edit],
    pub(super) orders_members: bool,
    pub(super) sort_definitions: bool,
    pub(super) source: &'a Source,
}

/// Computes the reorder of `body`: renders each member, then permutes the
/// slots through [`seat_body`], leaving the assembly to the caller. A
/// member a suppression pins keeps its text, and the rewrites inside it
/// land in `held` per [`held_edits`].
pub(super) fn body_layout<'a>(
    ctx: RewriteCtx<'a>,
    body: &'a [Stmt],
    outer: TextRange,
    scope: BodyScope,
) -> BodyLayout<'a> {
    let source = ctx.source;
    let mut held: Vec<Edit> = Vec::new();
    let mut pinned_starts: Vec<TextSize> = Vec::new();
    let Assembly {
        blocks,
        mut order,
        rendered,
    } = rendered_member_blocks(source, body, outer, |stmt, block| {
        if pins(source, block) {
            pinned_starts.push(stmt.start());
            held.extend(held_edits(ctx, stmt, block, scope));
            return Cow::Borrowed(source.slice(block));
        }
        rewrite_stmt(ctx, stmt, block, scope)
    });
    let import_run_slots = seat_body(ctx, body, &blocks, &mut order, scope, &pinned_starts);
    BodyLayout {
        assembly: Assembly {
            blocks,
            order,
            rendered,
        },
        held,
        import_run_slots,
    }
}

/// Returns the rows of the body the class `stmt` opens, keyed by the
/// start of its first statement, wherever the sort seats them other than
/// as written, each slot in the order the sort seats it beside whether it
/// opens on the line directly below the slot before it, per
/// [`seated_rows`].
pub(super) fn class_rows<'a>(
    ctx: RewriteCtx<'a>,
    stmt: &'a Stmt,
) -> Option<(TextSize, Vec<(usize, bool)>)> {
    let (body, scope) = scoped_body(stmt)?;
    let first = body.first()?;
    let ctx = scoped_ctx(ctx, stmt);
    let source = ctx.source;
    let blocks = member_blocks(source, body, stmt.range());
    let pinned_starts: Vec<TextSize> = body
        .iter()
        .zip(&blocks)
        .filter(|&(_, &block)| pins(source, block))
        .map(|(member, _)| member.start())
        .collect();
    let mut order: Vec<usize> = (0..body.len()).collect();
    let import_run_slots = seat_body(ctx, body, &blocks, &mut order, scope, &pinned_starts);
    let rows = seated_rows(
        source,
        body,
        &blocks,
        &order,
        |i| import_gap(source, &import_run_slots, i),
        |slot| source.slice(blocks[slot]),
    )?;
    Some((first.start(), rows))
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

/// Returns the edits the rewrite makes inside `block`, the block of a
/// statement a suppression pins, which keeps its text everywhere else.
/// Each sub-body contributes the edits its own layout makes, and each
/// stretch of `block` between sub-bodies contributes its leaf edits as
/// one narrowed edit, so no edit reaches text a sub-body pins.
fn held_edits(
    ctx: RewriteCtx<'_>,
    stmt: &Stmt,
    block: TextRange,
    parent_scope: BodyScope,
) -> Vec<Edit> {
    let source = ctx.source;
    let (body_ctx, scope) = scoped_body(stmt).map_or((ctx, parent_scope), |(_, scope)| {
        (scoped_ctx(ctx, stmt), scope)
    });
    let mut edits = Vec::new();
    let mut cursor = block.start();
    let mut stretches = Vec::new();
    for (body, outer) in sub_bodies(stmt)
        .into_iter()
        .filter(|(body, _)| !body.is_empty())
    {
        let layout = body_layout(body_ctx, body, outer, scope);
        let span = blocks_span(&layout.assembly.blocks);
        stretches.push(TextRange::new(cursor, span.start()));
        cursor = span.end();
        let forced = !layout.import_run_slots.is_empty();
        edits.extend(
            layout
                .assembly
                .cell_edits(source, forced, |i| {
                    import_gap(source, &layout.import_run_slots, i)
                })
                .into_iter()
                .flatten(),
        );
        edits.extend(layout.held);
    }
    stretches.push(TextRange::new(cursor, block.end()));
    edits.extend(stretches.into_iter().filter_map(|stretch| {
        narrowed_replacement(
            source,
            stretch,
            apply_inline_edits(source, stretch, ctx.leaf_edits),
        )
    }));
    edits
}

/// Reports whether a suppression pins `block` for `alphabetize-siblings`.
fn pins(source: &Source, block: TextRange) -> bool {
    source
        .suppression_map()
        .pins(block, AlphabetizeSiblings::SLUG)
}

/// Rewrites a non-empty body, returning the rewritten text alongside
/// the block-extent span it covers. The text is `Cow::Owned` when any
/// sibling reorder fires, any descendant rewrite produces owned
/// content, or any leaf edit lands inside, falling back to
/// `Cow::Borrowed` over `source.slice(span)`. `scope` selects which
/// family sorts apply. A pinned member pins every block holding it, so
/// a body this path reaches holds none.
fn rewrite_body<'a>(
    ctx: RewriteCtx<'a>,
    body: &'a [Stmt],
    outer: TextRange,
    scope: BodyScope,
) -> (Cow<'a, str>, TextRange) {
    let layout = body_layout(ctx, body, outer, scope);
    debug_assert!(
        layout.held.is_empty(),
        "a body under an unpinned statement holds no pinned member",
    );
    layout
        .assembly
        .or_borrow(ctx.source, !layout.import_run_slots.is_empty(), |i| {
            import_gap(ctx.source, &layout.import_run_slots, i)
        })
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
    let ctx = scoped_ctx(ctx, stmt);
    let (body_text, body_span) = rewrite_body(ctx, body, stmt.range(), scope);
    splice_bodies(ctx.source, block, [(body_text, body_span)], ctx.leaf_edits)
}

/// Returns `ctx` for the body the class or function `stmt` opens, with
/// `keeps_order` taken per [`sub_bodies_keep_order`] and, for a class,
/// the keyword-field offset and the member-order flag its header sets.
fn scoped_ctx<'a>(ctx: RewriteCtx<'a>, stmt: &Stmt) -> RewriteCtx<'a> {
    let ctx = RewriteCtx {
        keeps_order: sub_bodies_keep_order(ctx.source, stmt, ctx.keeps_order),
        ..ctx
    };
    stmt.as_class_def_stmt().map_or(ctx, |class| RewriteCtx {
        keyword_fields_from: keyword_field_start(class),
        orders_members: class_orders_members(class, ctx.enumerations),
        ..ctx
    })
}

/// Permutes `order`, seeded to source order over `body`'s member
/// `blocks`, within each section by the family sorts and import grouping
/// that `scope` enables, and returns the new-order slots whose import
/// neighbor collapses onto one line. The section partition walls each
/// notebook cell, so no permutation crosses a cell. A body under
/// `keeps_order` skips every permutation while its import neighbors still
/// collapse. A member whose start `pinned_starts` holds keeps its slot
/// while the definitions around it sort past it and the imports on either
/// side sort apart, and the gaps beside it stay as written.
fn seat_body<'a>(
    ctx: RewriteCtx<'a>,
    body: &'a [Stmt],
    blocks: &[TextRange],
    order: &mut Vec<usize>,
    scope: BodyScope,
    pinned_starts: &[TextSize],
) -> Vec<usize> {
    let RewriteCtx {
        defer_annotations,
        first_party,
        group_imports,
        group_methods,
        keeps_order,
        keyword_fields_from,
        orders_members,
        sort_definitions,
        source,
        ..
    } = ctx;
    let pinned = |stmt: &Stmt| pinned_starts.binary_search(&stmt.start()).is_ok();
    let pinned_slots: Vec<usize> = body.iter().positions(pinned).collect();
    let in_class = scope == BodyScope::Class;
    let mut import_run_slots: Vec<usize> = Vec::new();
    if !any_sibling_shares_line(source, body) {
        let sections = Sections::of(source, blocks);
        if !keeps_order && scope != BodyScope::Function {
            let holds = |stmt: &Stmt| (!in_class && is_decorated(stmt)) || pinned(stmt);
            let refs = eval_time_refs_of(body, defer_annotations);
            let defined = definition_names(body);
            let reachable = if consults_call_graph(body, &refs, &defined) {
                call_reachable(source.binding_analysis(), body)
            } else {
                CallReach::default()
            };
            let evaluated = Evaluated::of(body, &reachable, refs);
            let evaluation = evaluated.evaluation();
            let fences = fenced_slots(body, &defined);
            let prepared: Vec<SectionRuns<'_, 'a>> = sections
                .ranges()
                .iter()
                .flat_map(|section| fenced_runs(section, &fences))
                .map(|section| {
                    SectionRuns::of(
                        body,
                        section,
                        evaluation,
                        in_class,
                        group_methods,
                        orders_members,
                        sort_definitions,
                    )
                })
                .collect();
            // A permutation reverted for a reference that a later
            // permutation relocates becomes legal once that one lands, so
            // the section's permutations run to a fixed point. Each run
            // tiers once ahead of the loop, only the arrangement changing
            // per pass.
            let mut settled: Vec<usize> = Vec::with_capacity(order.len());
            for _ in 0..body.len().max(1) {
                settled.clear();
                settled.extend_from_slice(order);
                for section in &prepared {
                    section.permute(order, body, holds, keyword_fields_from);
                }
                if *order == settled {
                    break;
                }
            }
        }
        if !keeps_order {
            permute_runs(
                order,
                body,
                sectioned_import_runs(&sections, body)
                    .iter()
                    .flat_map(|run| fenced_runs(run, &pinned_slots)),
                |s| import_sort_key(s, first_party, group_imports),
            );
        }
        // Same-group import neighbors collapse to one line, except across a
        // section marker or beside a pinned member. A slot gap holding a
        // comment and a member block opening on a bound run both keep
        // their source gap.
        import_run_slots = adjacent_slots(order, |slot, a, b| {
            import_blank_lines(&body[a], &body[b], first_party, group_imports) == Some(0)
                && !sections.is_boundary(slot + 1)
                && !pinned(&body[a])
                && !pinned(&body[b])
                && source
                    .comment_ranges()
                    .comments_in_range(TextRange::new(blocks[slot].end(), blocks[slot + 1].start()))
                    .is_empty()
                && blocks[b].start() == source.text().line_start(body[b].start())
        });
    }
    import_run_slots
}
