//! The writing half of the `reflow-collections` walker: the expanded
//! and rejoined text a construct is replaced by, each child serialized
//! where its row lands.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_text_size::{Ranged, TextRange};

use super::{
    Layouter,
    classify::{Segment, is_atomic, segments},
    entry_tail,
    flow::{Packing, flow_lines},
};
use crate::{
    primitives::{
        inline::settled_text_width,
        layout::item_indent,
        travel::{Landing, placed_block},
    },
    rules::reflow_calls::Reshaper,
};

/// Per-item state for a dict, list, set, or tuple literal: serialized
/// text, atomicity for layout dispatch, source range for blank-line
/// lookups, and display width at the canonical `": "` separator.
struct GatheredItems<'src> {
    atomics: Vec<bool>,
    close: char,
    open: char,
    ranges: Vec<TextRange>,
    texts: Vec<Cow<'src, str>>,
    widths: Vec<usize>,
}

impl<'src> GatheredItems<'src> {
    /// The items between `open` and `close`, each read as its text,
    /// width, atomicity, and source range.
    fn of(
        open: char,
        close: char,
        items: impl Iterator<Item = (Cow<'src, str>, usize, bool, TextRange)>,
    ) -> Self {
        let (texts, widths, atomics, ranges) = items.multiunzip();
        Self {
            atomics,
            close,
            open,
            ranges,
            texts,
            widths,
        }
    }
}

impl<'a> Layouter<'a> {
    /// Collects the bracket pair and per-item text, atomicity, and source
    /// range for the collection at `expr`, each child serialized through
    /// `serialize_expr` or `gather_entries` at `indent` and charged the
    /// separator the sort `order` names leaves closing its row, a dict's
    /// entries read in that order too. An item needing neither a rewrite
    /// nor a move borrows its source slice.
    fn gather_items(
        &self,
        expr: &Expr,
        indent: usize,
        order: Option<&[usize]>,
    ) -> GatheredItems<'a> {
        let node = AnyNodeRef::from(expr);
        let tail = |last: Option<TextRange>| {
            move |i: usize, count: usize, entry: TextRange| {
                entry_tail(last, entry, usize::from(i + 1 < count))
            }
        };
        let (open, close, elts) = match expr {
            Expr::Dict(d) => {
                let tail = tail(sorted_last(&d.items, order));
                return GatheredItems::of('{', '}', self.gather_entries(d, indent, order, tail));
            }
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            Expr::Tuple(t) => ('(', ')', &t.elts),
            _ => unreachable!("gather_items called on non-expandable expr"),
        };
        let tail = tail(sorted_last(elts, order));
        GatheredItems::of(
            open,
            close,
            elts.iter().enumerate().map(|(i, e)| {
                let tail = tail(i, elts.len(), e.range());
                let text = self.serialize_expr(e, node, indent, indent, tail);
                let width = settled_text_width(
                    self.source,
                    self.padding,
                    &text,
                    self.range_with_parens(e, node),
                );
                (text, width, is_atomic(e), e.range())
            }),
        )
    }

    /// Builds the expanded form of `expr` under `parent` as a string,
    /// recursively laying out any qualifying child collections. Every
    /// row is charged the separator a later sort leaves closing it, and
    /// a flowed row packs the entries that sort leaves in it.
    pub(super) fn expand(&self, expr: &Expr, parent: AnyNodeRef, indent: usize) -> String {
        let item_indent = item_indent(indent);
        let node = AnyNodeRef::from(expr);
        let order = self.reorders.sorted_slots(self.source, node, parent);
        let GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
            widths,
        } = self.gather_items(expr, item_indent, order.as_deref());
        let total = texts.len();
        let item_prefix = " ".repeat(item_indent);
        let available = self.code_line_length.saturating_sub(item_indent);
        let mut out = String::new();
        out.push(open);
        out.push_str(self.newline);
        for segment in segments(&atomics, expr.is_set_expr()) {
            match segment {
                Segment::Flow(range) => {
                    let run_start = range.start;
                    let packing = Packing {
                        available,
                        followed: range.end < total,
                        max_atomics: self.max_atomics,
                    };
                    // The slots pack at the widths of the entries the sort
                    // leaves in them.
                    let slot_widths: Vec<usize> = match &order {
                        Some(order) => order[range].iter().map(|&index| widths[index]).collect(),
                        None => widths[range].to_vec(),
                    };
                    for line_range in flow_lines(&slot_widths, packing) {
                        let line_start = run_start + line_range.start;
                        let line_end = run_start + line_range.end;
                        out.push_str(&item_prefix);
                        out.push_str(&texts[line_start..line_end].join(", "));
                        if line_end < total {
                            out.push(',');
                        }
                        out.push_str(self.newline);
                    }
                }
                Segment::OnePerLine(range) => {
                    for idx in range {
                        let has_more = idx + 1 < total;
                        out.push_str(&item_prefix);
                        out.push_str(&texts[idx]);
                        if has_more {
                            out.push(',');
                        }
                        out.push_str(self.newline);
                        if has_more && self.source.has_blank_line_before(ranges[idx + 1].start()) {
                            out.push_str(self.newline);
                        }
                    }
                }
            }
        }
        out.push_str(&item_prefix[..indent]);
        out.push(close);
        out
    }

    /// `expr`'s paren-recovered source range placed per `landing`, the
    /// calls inside it reshaped where the move pushes one past the
    /// budget and the slice moved whole otherwise. `tail` is the columns
    /// the enclosing layout writes after the text on its last row.
    pub(super) fn placed_slice(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        landing: Landing,
        tail: usize,
    ) -> Cow<'a, str> {
        let range = self.range_with_parens(expr, parent);
        self.reshaper()
            .reshaped(expr, range, landing, tail)
            .map_or_else(|| placed_block(self.source, range, landing), Cow::Owned)
    }

    /// The one-line form of a fractured `expr`, or `None` when it holds
    /// no break or overflows the budget once joined. The repair runs
    /// whatever `keep_multiline_literals` holds, covering a subscript, a
    /// comprehension, and a dict key, whose breaks fall outside the entry
    /// boundaries the expand path lays a literal out on.
    pub(super) fn repaired(&self, expr: &Expr, column: usize, tail: usize) -> Option<String> {
        self.source
            .contains_line_break(expr.range())
            .then(|| {
                self.one_row
                    .repaired(self.source, expr, expr.into(), column, tail)
            })
            .flatten()
            .map(Cow::into_owned)
    }

    /// The terms the calls inside a relocated expression reshape under.
    pub(super) fn reshaper(&self) -> Reshaper<'a> {
        Reshaper {
            expands_literals: self.explode,
            one_row: self.one_row,
            padding: self.padding,
            reorders: self.reorders,
            reservations: self.reservations,
            source: self.source,
            targets: self.targets,
        }
    }
}

/// The range of the entry of `items` the sort leaves last, `None` where
/// `order` names no sort.
fn sorted_last<T: Ranged>(items: &[T], order: Option<&[usize]>) -> Option<TextRange> {
    order?.last().map(|&index| items[index].range())
}
