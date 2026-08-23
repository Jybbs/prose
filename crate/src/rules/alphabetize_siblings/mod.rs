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
//! block past a `KW_ONLY` sentinel to sort. A decorated definition holds
//! its source slot at module scope and sorts inside a class body.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, ArgOrKeyword, Arguments, Expr, Stmt, helpers::is_compound_statement,
};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        binding::{sequence_elts, single_name_target},
        comments::has_keep_marker,
        constructor::keyword_field_start,
        decorator::is_decorated,
        edit::{apply_inline_edits, singleton_groups, splice_bodies},
        effect::value_is_effectful,
        imports::{defers_annotations, import_blank_lines, import_sort_key, sectioned_import_runs},
        orderer::{
            adjacent_slots, any_sibling_shares_line, assemble_or_borrow, assembled_cell_edits,
            opens_its_line, permute_runs, rendered_member_blocks, swap_span_commented,
        },
        scope::{BodyScope, compound_sub_bodies, scoped_body},
        sections::Sections,
        slots::runs_where,
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
    dict::dict_sort_key,
    leaves::{collect_docstring_entry_edits, collect_leaf_edits},
    members::{class_pins_methods, method_group},
};

/// The leaf sorts `alphabetize-siblings` will make, forecast by a rule
/// seated ahead of it that measures an entry with the separator the
/// sort leaves after it rather than the one it carries now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Reorders {
    enabled: bool,
    sort_dict_keys: bool,
    sort_dunder_lists: bool,
}

impl Reorders {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.alphabetize_siblings;
        Self {
            enabled: rules.enabled,
            sort_dict_keys: rules.sort_dict_keys,
            sort_dunder_lists: rules.sort_dunder_lists,
        }
    }

    /// The range of the entry of `node` the sort leaves last, `None`
    /// where no entry of `node` sorts per [`Self::sorted_slots`]. A
    /// pinned last entry stays last, and otherwise the greatest key in
    /// the run closing there lands last.
    pub(crate) fn sorted_last(
        self,
        source: &Source,
        node: AnyNodeRef,
        parent: AnyNodeRef,
    ) -> Option<TextRange> {
        let sorted = self.sorted(source, node, parent)?;
        Some(sorted.ranges[*sorted.order.last()?])
    }

    /// The order the sort leaves `node`'s entries in, each slot holding
    /// the source index of the entry landing there, `None` where no
    /// entry of `node` sorts: the rule off or skip-held over `node`, a
    /// node of another kind, a list or tuple bound to nothing the rule
    /// sorts under `parent`, a dict or dunder list held by
    /// `# prose: keep` or the config, or fewer than two entries.
    pub(crate) fn sorted_slots(
        self,
        source: &Source,
        node: AnyNodeRef,
        parent: AnyNodeRef,
    ) -> Option<Vec<usize>> {
        Some(self.sorted(source, node, parent)?.order)
    }

    /// The sort over `node`'s entries, per [`Self::sorted_slots`].
    fn sorted(self, source: &Source, node: AnyNodeRef, parent: AnyNodeRef) -> Option<Sorted> {
        if !self.sorts(source, node) {
            return None;
        }
        match node {
            AnyNodeRef::Arguments(arguments) => {
                let items: Vec<ArgOrKeyword> = arguments.iter_source_order().collect();
                // A `**` unpacking bounds each run the keywords sort within,
                // whereas a positional argument pins in place inside one.
                let inside =
                    |arg: &ArgOrKeyword| arg.as_keyword().is_none_or(|kw| kw.arg.is_some());
                sorted_entries(&items, runs_where(&items, inside), |arg| {
                    arg.as_keyword()?
                        .arg
                        .as_deref()
                        .filter(|_| !value_is_effectful(arg.value()))
                })
            }
            AnyNodeRef::ExprDict(dict) if self.sort_dict_keys && !has_keep_marker(source, dict) => {
                let keyed = runs_where(&dict.items, |item| item.key.is_some());
                sorted_entries(&dict.items, keyed, |item| dict_sort_key(source, item))
            }
            AnyNodeRef::ExprSet(set) => sorted_entries(&set.elts, whole(&set.elts), |e| {
                (!e.is_starred_expr()).then(|| joined_key(source, e))
            }),
            AnyNodeRef::ExprList(_) | AnyNodeRef::ExprTuple(_)
                if self.sort_dunder_lists && !has_keep_marker(source, node) =>
            {
                let AnyNodeRef::StmtAssign(assign) = parent else {
                    return None;
                };
                let elts = sequence_elts(&assign.value)
                    .filter(|_| matches!(single_name_target(assign), Some("__all__" | "__slots__")))
                    .filter(|_| assign.value.range() == node.range())?;
                sorted_entries(elts, whole(elts), |e| {
                    Some(e.as_string_literal_expr()?.value.to_str())
                })
            }
            _ => None,
        }
    }

    /// True when the rule reaches `node` at all, on and not held by a
    /// skip directive over it, the same hold the pipeline applies to the
    /// rule's own fix group.
    fn sorts(self, source: &Source, node: impl Ranged) -> bool {
        self.enabled
            && !source
                .suppression_map()
                .suppresses(node, AlphabetizeSiblings::SLUG)
    }

    /// True when the sort leaves `node` as laid out, the gates the rule
    /// reads off the layout rather than the entries: a multi-line dict
    /// packing entries onto a shared row, one whose first entry trails
    /// the `{` on its row with a comment in the span, and a multi-line
    /// set, list, or tuple packing entries or opening mid-row with a
    /// comment in the span.
    pub(crate) fn holds_as_laid_out(self, source: &Source, node: AnyNodeRef) -> bool {
        fn packed<T: Ranged>(source: &Source, items: &[T]) -> (bool, bool, bool) {
            let [first, .., last] = items else {
                return (false, false, false);
            };
            let span = TextRange::new(first.start(), last.end());
            (
                source.contains_line_break(span),
                any_sibling_shares_line(source, items),
                !opens_its_line(source, first.start()) && swap_span_commented(source, items),
            )
        }
        match node {
            AnyNodeRef::ExprDict(dict) => {
                let (multi_line, shares, mid_row_commented) = packed(source, &dict.items);
                multi_line && (shares || mid_row_commented)
            }
            AnyNodeRef::ExprList(list) => held_leaves(source, &list.elts),
            AnyNodeRef::ExprSet(set) => held_leaves(source, &set.elts),
            AnyNodeRef::ExprTuple(tuple) => held_leaves(source, &tuple.elts),
            _ => false,
        }
    }

    /// The index of the row the sort leaves last among keyword `rows`,
    /// each a name and its value, a row binding an effectful value
    /// pinned in its slot and the rest sorted by name. `None` where the
    /// rule is off or skip-held over `arguments`.
    pub(crate) fn sorted_last_keyword<'a>(
        self,
        source: &Source,
        arguments: &Arguments,
        rows: impl Iterator<Item = (&'a str, &'a Expr)>,
    ) -> Option<usize> {
        if !self.sorts(source, arguments) {
            return None;
        }
        let keys: Vec<Option<&str>> = rows
            .map(|(name, value)| (!value_is_effectful(value)).then_some(name))
            .collect();
        sorted_order(&keys, whole(&keys))?.pop()
    }
}

/// The sort over one node's entries: the source index landing in each
/// slot, and every entry's range in source order.
struct Sorted {
    order: Vec<usize>,
    ranges: Vec<TextRange>,
}

/// True when a leaf group over `items` holds its order as laid out: one
/// packing members onto shared rows or opening mid-row, spanning lines,
/// with a comment inside the swap span.
fn held_leaves<T: Ranged>(source: &Source, items: &[T]) -> bool {
    let [first, .., last] = items else {
        return false;
    };
    (any_sibling_shares_line(source, items) || !opens_its_line(source, first.start()))
        && source.contains_line_break(TextRange::new(first.start(), last.end()))
        && swap_span_commented(source, items)
}

/// `ranged`'s source text read the way a later join writes it onto one
/// row, per [`joined_text`], so a fractured element sorts where its
/// joined form will.
pub(super) fn joined_key<'a>(source: &'a Source, ranged: impl Ranged) -> Cow<'a, str> {
    joined_text(source.slice(ranged.range()))
}

/// `slice` read the way a later join writes it onto one row, every
/// whitespace run one space, none directly inside a bracket, and no
/// comma ahead of a closer. A single-line slice passes through
/// borrowed.
pub(super) fn joined_text(slice: &str) -> Cow<'_, str> {
    if !slice.contains('\n') {
        return Cow::Borrowed(slice);
    }
    let mut out = String::with_capacity(slice.len());
    for word in slice.split_whitespace() {
        if word.starts_with([')', ']', '}']) {
            while out.ends_with(',') {
                out.pop();
            }
        } else if !out.is_empty() && !out.ends_with(['(', '[', '{']) {
            out.push(' ');
        }
        out.push_str(word);
    }
    Cow::Owned(out)
}

/// The sort of `items` under `key` over `runs`, `None` where fewer than
/// two are present.
fn sorted_entries<'a, T: Ranged, K: Ord>(
    items: &'a [T],
    runs: impl IntoIterator<Item = Range<usize>>,
    key: impl FnMut(&'a T) -> Option<K>,
) -> Option<Sorted> {
    let keys: Vec<Option<K>> = items.iter().map(key).collect();
    Some(Sorted {
        order: sorted_order(&keys, runs)?,
        ranges: items.iter().map(Ranged::range).collect(),
    })
}

/// The slot order the sort leaves `keys` in, each run sorted on its own
/// with a keyless slot pinned in place and equal keys keeping their
/// order, `None` for fewer than two slots.
fn sorted_order<K: Ord>(
    keys: &[Option<K>],
    runs: impl IntoIterator<Item = Range<usize>>,
) -> Option<Vec<usize>> {
    if keys.len() < 2 {
        return None;
    }
    let mut order: Vec<usize> = (0..keys.len()).collect();
    permute_runs(&mut order, keys, runs, Option::as_ref);
    Some(order)
}

/// One run covering every slot of `items`.
fn whole<T>(items: &[T]) -> Option<Range<usize>> {
    Some(0..items.len())
}

pub(crate) struct AlphabetizeSiblings {
    code_width: usize,
    first_party: Vec<String>,
    group_imports: bool,
    group_methods: bool,
    sort_definitions: bool,
    sort_dict_keys: bool,
    sort_docstring_entries: bool,
    sort_dunder_lists: bool,
}

impl AlphabetizeSiblings {
    pub(crate) const MESSAGE: &'static str = "alphabetize this group";

    pub(crate) fn from_config(config: &Config) -> Self {
        let alphabetize_siblings = &config.rules.alphabetize_siblings;
        Self {
            code_width: config.code_width(),
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            group_methods: alphabetize_siblings.group_methods,
            sort_definitions: alphabetize_siblings.sort_definitions,
            sort_dict_keys: alphabetize_siblings.sort_dict_keys,
            sort_docstring_entries: alphabetize_siblings.sort_docstring_entries,
            sort_dunder_lists: alphabetize_siblings.sort_dunder_lists,
        }
    }
}

impl Rule for AlphabetizeSiblings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let mut leaf_edits = collect_leaf_edits(
            source,
            self.code_width,
            self.sort_dict_keys,
            self.sort_dunder_lists,
        );
        if self.sort_docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source));
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
            let holds = |stmt: &Stmt| !in_class && is_decorated(stmt);
            for section in sections.ranges() {
                if sort_definitions {
                    permute_defs(
                        &mut order,
                        body,
                        section.clone(),
                        defer_annotations,
                        holds,
                        |s| {
                            s.as_class_def_stmt().map(|c| {
                                let name = c.name.as_str();
                                (name, name)
                            })
                        },
                    );
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
                if sort_definitions && !(in_class && class_pins_methods(&body[section.clone()])) {
                    permute_defs(
                        &mut order,
                        body,
                        section.clone(),
                        defer_annotations,
                        holds,
                        |s| {
                            s.as_function_def_stmt().map(|f| {
                                let name = f.name.as_str();
                                let group = if group_methods { method_group(f) } else { 0 };
                                (name, (group, name))
                            })
                        },
                    );
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
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, first_value, parse};

    #[rstest]
    #[case::packed_commented_list("x = [b, a,  # c\n     d]\n", true)]
    #[case::packed_commented_tuple("x = (b, a,  # c\n     d)\n", true)]
    #[case::packed_commented_set("x = {b, a,  # c\n     d}\n", true)]
    #[case::one_per_line_commented("x = [\n    b,  # c\n    a,\n]\n", false)]
    #[case::single_element("x = [b]\n", false)]
    #[case::packed_commented_dict("x = {\"b\": 1, \"a\": 2,  # c\n     \"d\": 3}\n", true)]
    #[case::single_entry_dict("x = {\"b\": 1}\n", false)]
    fn holds_as_laid_out_reads_the_layout_gates(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let reorders = Reorders::from_config(&Config::default());
        assert_eq!(
            reorders.holds_as_laid_out(&source, first_value(&source).into()),
            expected
        );
    }

    #[test]
    fn apply_skips_dict_key_reorder_when_config_disables_it() {
        let src = "row = {\"model\": 1, \"epochs\": 2}\ntags = {\"zeta\", \"alpha\"}\n";
        let mut config = Config::default();
        config.rules.alphabetize_siblings.sort_dict_keys = false;
        let rule = AlphabetizeSiblings::from_config(&config);
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
        config.rules.alphabetize_siblings.sort_docstring_entries = false;
        let rule = AlphabetizeSiblings::from_config(&config);
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
