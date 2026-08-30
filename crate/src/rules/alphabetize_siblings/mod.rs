//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning: classes and functions in a body, class-scope field runs,
//! keyword-only parameters, call kwargs, dict keys, set elements,
//! import names and alias lists within each section, `global` /
//! `nonlocal` / `del` name lists, and the strings inside `__all__` /
//! `__slots__`. Sorting flows through the `primitives::orderer`
//! permute and assemble primitives, a recursive rewriter folding inner
//! sorts into the outer scope's replacement text so each outermost
//! scope emits one edit, or one per notebook cell. Positional-or-
//! keyword parameters never reorder and only the keyword-only block
//! past `*` sorts, a class whose header generates a field-ordered
//! constructor holds its field run, and a decorated definition holds
//! its slot at module scope while sorting inside a class body.

use std::{borrow::Cow, ops::Range};

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, ArgOrKeyword, Arguments, Expr};
use ruff_text_size::{Ranged, TextRange, TextSize};

use self::{
    dict::dict_sort_key,
    leaves::{collect_docstring_entry_edits, collect_leaf_edits},
    rewrite::{RewriteCtx, body_layout, import_gap},
};
use crate::{
    config::Config,
    primitives::{
        binding::{sequence_elts, single_name_target},
        comments::has_keep_marker,
        effect::value_is_effectful,
        imports::defers_annotations,
        orderer::{
            any_sibling_shares_line, assembled_cell_edits, opens_its_line, permute_runs,
            swap_span_commented,
        },
        scope::BodyScope,
        slots::runs_where,
        tokens::{CLOSERS, OPENERS},
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod class_graph;
mod dict;
mod leaves;
mod members;
mod module_graph;
mod rewrite;

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
        assembled_cell_edits(
            source,
            &layout.blocks,
            &layout.rendered,
            &layout.order,
            !layout.import_run_slots.is_empty(),
            |i| import_gap(source, &layout.import_run_slots, i),
        )
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

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
fn joined_key(source: &Source, ranged: impl Ranged) -> Cow<'_, str> {
    joined_text(source.slice(ranged.range()))
}

/// `slice` read the way a later join writes it onto one row, every
/// whitespace run one space, none directly inside a bracket, and no
/// comma ahead of a closer. A single-line slice passes through
/// borrowed.
fn joined_text(slice: &str) -> Cow<'_, str> {
    if !slice.contains('\n') {
        return Cow::Borrowed(slice);
    }
    let mut out = String::with_capacity(slice.len());
    for word in slice.split_whitespace() {
        if word.starts_with(CLOSERS) {
            while out.ends_with(',') {
                out.pop();
            }
        } else if !out.is_empty() && !out.ends_with(OPENERS) {
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

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, first_value, parse};

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
}
