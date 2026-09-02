//! The leaf sorts `alphabetize-siblings` will make, forecast by a rule
//! seated ahead of it so it measures an entry with the separator the
//! sort leaves after it rather than the one it carries now.

use std::{borrow::Cow, ops::Range};

use ruff_python_ast::{AnyNodeRef, ArgOrKeyword, Arguments, Expr};
use ruff_text_size::{Ranged, TextRange};

use super::dict::{dict_holds_as_laid_out, dict_sort_key};
use super::{AlphabetizeSiblings, dunder_list};
use crate::{
    config::Config,
    primitives::{
        binding::sequence_elts,
        comments::has_keep_marker,
        effect::value_is_effectful,
        inline::spans_rows,
        orderer::{permute_runs, swap_span_holds, swaps_in_place},
        slots::runs_where,
        tokens::{CLOSERS, OPENERS},
    },
    source::Source,
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
                    .filter(|_| dunder_list(assign))
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
        match node {
            AnyNodeRef::ExprDict(dict) => dict_holds_as_laid_out(source, &dict.items),
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
        Some(self.sorted(source, node, parent)?.last)
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

/// The sort over one node's entries: the range of the entry the sort
/// leaves last, and the source index landing in each slot.
struct Sorted {
    last: TextRange,
    order: Vec<usize>,
}

/// `ranged`'s source text read the way a later join writes it onto one
/// row, per [`joined_text`], so a fractured element sorts where its
/// joined form will.
pub(super) fn joined_key(source: &Source, ranged: impl Ranged) -> Cow<'_, str> {
    joined_text(source.slice(ranged.range()))
}

/// `slice` read the way a later join writes it onto one row, every
/// whitespace run one space, none directly inside a bracket, and no
/// comma ahead of a closer. A single-line slice passes through
/// borrowed.
pub(super) fn joined_text(slice: &str) -> Cow<'_, str> {
    if !spans_rows(slice) {
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

/// True when a leaf group over `items` holds its order as laid out: one
/// packing members onto shared rows or opening mid-row, spanning lines,
/// with a comment inside the swap span.
fn held_leaves<T: Ranged>(source: &Source, items: &[T]) -> bool {
    swap_span_holds(source, items, swaps_in_place(source, items))
}

/// The sort of `items` under `key` over `runs`, `None` where fewer than
/// two are present.
fn sorted_entries<'a, T: Ranged, K: Ord>(
    items: &'a [T],
    runs: impl IntoIterator<Item = Range<usize>>,
    key: impl FnMut(&'a T) -> Option<K>,
) -> Option<Sorted> {
    let keys: Vec<Option<K>> = items.iter().map(key).collect();
    let order = sorted_order(&keys, runs)?;
    Some(Sorted {
        last: items[*order.last()?].range(),
        order,
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
