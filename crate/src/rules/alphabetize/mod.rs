//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning. The covered shapes are classes and functions in a body,
//! class-scope `Stmt::AnnAssign` field declarations and `Stmt::Assign`
//! runs with simple `Name` targets, function and lambda keyword-only
//! parameters, call kwargs, dict-literal keys, set literal elements,
//! import names and their alias lists within each section, `global` and
//! `nonlocal` name lists, `del` target lists, and the string literals
//! inside `__all__` / `__slots__`.
//!
//! Sorting flows through the `primitives::orderer` permute and assemble
//! primitives. A recursive `Cow<'src, str>` rewriter folds inner sorts
//! into the outer scope's replacement text, so each outermost reordering
//! scope emits a single edit covering its descendants, or one edit per
//! cell over a notebook.
//!
//! Positional-or-keyword parameters never reorder, free function and
//! method alike, because no single-file rewrite can keep every caller's
//! positional binding intact. Only the keyword-only block past `*`
//! sorts. A class whose header generates a field-ordered constructor
//! holds its annotated field run for that same reason, leaving the
//! block past a `KW_ONLY` sentinel to sort.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, helpers::is_compound_statement};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        constructor::keyword_field_start,
        edit::{apply_inline_edits, singleton_groups, splice_bodies},
        imports::{defers_annotations, import_blank_lines, import_sort_key, sectioned_import_runs},
        orderer::{
            adjacent_slots, any_sibling_shares_line, assemble_or_borrow, assembled_cell_edits,
            permute_runs, rendered_member_blocks,
        },
        scope::{BodyScope, compound_sub_bodies, scoped_body},
        sections::Sections,
        tiering::permute_defs,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod class_graph;
mod dict;
mod leaves;
mod members;

use self::{
    class_graph::permute_class_assigns,
    leaves::{collect_docstring_entry_edits, collect_leaf_edits},
    members::{class_pins_methods, method_group},
};

pub(crate) struct Alphabetize {
    first_party: Vec<String>,
    group_imports: bool,
    group_methods: bool,
    sort_definitions: bool,
    sort_dict_keys: bool,
    sort_docstring_entries: bool,
    sort_dunder_lists: bool,
}

impl Alphabetize {
    pub(crate) fn from_config(config: &Config) -> Self {
        let alphabetize = &config.rules.alphabetize;
        Self {
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            group_methods: alphabetize.group_methods,
            sort_definitions: alphabetize.sort_definitions,
            sort_dict_keys: alphabetize.sort_dict_keys,
            sort_docstring_entries: alphabetize.sort_docstring_entries,
            sort_dunder_lists: alphabetize.sort_dunder_lists,
        }
    }
}

impl Rule for Alphabetize {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let (mut leaf_edits, param_docs) =
            collect_leaf_edits(source, self.sort_dict_keys, self.sort_dunder_lists);
        if self.sort_docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source, &param_docs));
            leaf_edits.sort_unstable();
        }
        let ctx = RewriteCtx {
            defer_annotations: defers_annotations(body),
            first_party: &self.first_party,
            group_imports: self.group_imports,
            group_methods: self.group_methods,
            keyword_fields_from: TextSize::default(),
            leaf_edits: &leaf_edits,
            sort_definitions: self.sort_definitions,
            source,
        };
        let layout = body_layout(ctx, body, source.module_range(), BodyScope::Module);
        let edits = assembled_cell_edits(
            source,
            &layout.blocks,
            &layout.rendered,
            &layout.order,
            !layout.import_run_slots.is_empty(),
            |i| import_gap(&layout.import_run_slots, i),
        );
        singleton_groups(edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The reorder layout of one body: its member blocks, their rendered
/// text, the new-order permutation, and the new-order slots whose import
/// neighbor collapses onto one line. [`rewrite_body`] folds it into the
/// combined `Cow` and the notebook path splits it per cell.
struct BodyLayout<'a> {
    blocks: Vec<TextRange>,
    import_run_slots: Vec<usize>,
    order: Vec<usize>,
    rendered: Vec<Cow<'a, str>>,
}

/// Context threaded through the body-rewrite recursion, every field
/// invariant but `keyword_fields_from`, which each class header refreshes
/// for its own body.
#[derive(Clone, Copy)]
struct RewriteCtx<'a> {
    defer_annotations: bool,
    first_party: &'a [String],
    group_imports: bool,
    group_methods: bool,
    keyword_fields_from: TextSize,
    leaf_edits: &'a [Edit],
    sort_definitions: bool,
    source: &'a Source,
}

/// Computes the reorder of `body`: renders each member, then permutes the
/// slots within each section by the family sorts and import grouping that
/// `scope` enables, leaving the assembly to the caller. The section
/// partition walls each notebook cell, so no permutation crosses a cell.
fn body_layout<'a>(
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
            for section in sections.ranges() {
                let members = &body[section.clone()];
                if sort_definitions {
                    permute_defs(&mut order, body, section.clone(), defer_annotations, |s| {
                        s.as_class_def_stmt().map(|c| {
                            let name = c.name.as_str();
                            (name, name)
                        })
                    });
                }
                if in_class {
                    permute_class_assigns(
                        &mut order,
                        body,
                        section.clone(),
                        defer_annotations,
                        keyword_fields_from,
                    );
                }
                if sort_definitions && !(in_class && class_pins_methods(members)) {
                    permute_defs(&mut order, body, section.clone(), defer_annotations, |s| {
                        s.as_function_def_stmt().map(|f| {
                            let name = f.name.as_str();
                            let group = if group_methods { method_group(f) } else { 0 };
                            (name, (group, name))
                        })
                    });
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
        // section marker, whose dividing gap must survive in place.
        import_run_slots = adjacent_slots(&order, |slot, a, b| {
            import_blank_lines(&body[a], &body[b], first_party, group_imports) == Some(0)
                && !sections.is_boundary(slot + 1)
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
/// slot `i`, `None` where the neighbors do not collapse onto one line.
fn import_gap(import_run_slots: &[usize], i: usize) -> Option<&'static str> {
    import_run_slots.binary_search(&i).is_ok().then_some("\n")
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
        |i| import_gap(&layout.import_run_slots, i),
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

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::testing::{applied_text, parse};

    #[test]
    fn apply_skips_dict_key_reorder_when_config_disables_it() {
        let src = "row = {\"model\": 1, \"epochs\": 2}\ntags = {\"zeta\", \"alpha\"}\n";
        let mut config = Config::default();
        config.rules.alphabetize.sort_dict_keys = false;
        let rule = Alphabetize::from_config(&config);
        let source = parse(src);
        let edits = rule.apply(&source).into_iter().flatten().collect();
        assert_eq!(
            applied_text(&source, edits),
            "row = {\"model\": 1, \"epochs\": 2}\ntags = {\"alpha\", \"zeta\"}\n",
        );
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
        config.rules.alphabetize.sort_docstring_entries = false;
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
            "docstring entries should keep source order when sort-docstring-entries is off",
        );
    }
}
