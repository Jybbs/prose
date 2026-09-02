//! The writing half of the `reflow-collections` walker: the expanded
//! and rejoined text a construct is replaced by, each child serialized
//! where its row lands.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_python_ast::{AnyNodeRef, DictItem, Expr};
use ruff_text_size::{Ranged, TextRange};

use super::{
    CANONICAL_SEPARATOR, Layouter,
    classify::{Segment, is_align_colons_gap, is_atomic, pre_colon_padding, segments},
    entry_tail,
    flow::{Packing, flow_lines},
};
use crate::{
    primitives::{
        INDENT_STEP,
        inline::{display_width, end_column, settled_text_width},
        layout::item_indent,
        travel::{Landing, placed_block},
    },
    rules::{reflow_calls::Reshaper, stack_adjacent_strings::concatenated_run},
};

/// Per-item state for a dict, list, set, or tuple literal: serialized
/// text, atomicity for layout dispatch, source range for blank-line
/// lookups, and display width at the canonical `": "` separator, so an
/// `align_colons`-padded gap does not inflate the measure.
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
    /// range for the collection at `expr` under `parent`, each child
    /// serialized through `serialize_expr` / `serialize_dict_item` at
    /// `indent` so nested collections arrive already laid out, every one
    /// charged the separator a later sort leaves closing its row. An
    /// item needing neither a rewrite nor a move borrows its source
    /// slice.
    fn gather_items(&self, expr: &Expr, parent: AnyNodeRef, indent: usize) -> GatheredItems<'a> {
        let node = AnyNodeRef::from(expr);
        let last = self.reorders.sorted_last(self.source, node, parent);
        let tail = |i: usize, count: usize, entry: TextRange| {
            entry_tail(last, entry, usize::from(i + 1 < count))
        };
        let (open, close, elts) = match expr {
            Expr::Dict(d) => {
                return GatheredItems::of(
                    '{',
                    '}',
                    d.iter().enumerate().map(|(i, item)| {
                        let tail = tail(i, d.len(), item.range());
                        let (text, width) = self.serialize_dict_item(item, node, indent, tail);
                        (text, width, false, item.range())
                    }),
                );
            }
            Expr::List(l) => ('[', ']', &l.elts),
            Expr::Set(s) => ('{', '}', &s.elts),
            Expr::Tuple(t) => ('(', ')', &t.elts),
            _ => unreachable!("gather_items called on non-expandable expr"),
        };
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

    /// Builds the hung two-line form of a `key: value` dict entry,
    /// breaking at `:` and emitting the value at `item_indent +
    /// INDENT_STEP` with `tail` columns closing its row. The key routes
    /// through `repaired_key` the same way `serialize_dict_item` does,
    /// and its pre-colon padding carries through, the column belonging
    /// to `align_colons`. Returns `None` for a `**value` unpacking item
    /// and for an entry either side of whose `:` carries an implicitly
    /// concatenated string, which `stack-adjacent-strings` breaks in
    /// place.
    fn hang_dict_value(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        item_indent: usize,
        tail: usize,
    ) -> Option<String> {
        let key = item.key.as_ref()?;
        if concatenated_run(key).is_some() || concatenated_run(&item.value).is_some() {
            return None;
        }
        let key_text = self.repaired_key(key, parent, item_indent);
        let value_start = self.range_with_parens(&item.value, parent).start();
        let padding = pre_colon_padding(self.key_value_gap(key.end(), value_start));
        let hang_column = item_indent + INDENT_STEP;
        let value_text = self.serialize_expr(&item.value, parent, hang_column, hang_column, tail);
        let hang_prefix = " ".repeat(hang_column);
        Some(format!(
            "{key_text}{padding}:{newline}{hang_prefix}{value_text}",
            newline = self.newline,
        ))
    }

    /// Serializes a dict key, rejoining one written across lines so its
    /// `:` sits beside it and falling through to `serialize_expr`
    /// otherwise.
    fn repaired_key(&self, key: &Expr, parent: AnyNodeRef, indent: usize) -> Cow<'a, str> {
        self.repaired(key, indent, 0).map_or_else(
            || self.serialize_expr(key, parent, indent, indent, 0),
            Cow::Owned,
        )
    }

    /// Serializes a dict item as `key: value` or `**value`, paired with
    /// its display width at the canonical `": "` separator. The key
    /// routes through `repaired_key` so one written across lines rejoins
    /// beside its `:`, and the value's fit column sits past the key
    /// text's last row and the separator that lands ahead of it. A
    /// borrowed key and value
    /// over an `align-colons`-padded gap return the source slice whole so
    /// the padding round-trips, the width counting the canonical `": "`.
    fn serialize_dict_item(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        indent: usize,
        tail: usize,
    ) -> (Cow<'a, str>, usize) {
        let value_range = self.range_with_parens(&item.value, parent);
        let Some(key) = &item.key else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent, tail);
            let width = 2 + settled_text_width(self.source, self.padding, &value_text, value_range);
            return (Cow::Owned(format!("**{value_text}")), width);
        };
        let key_text = self.repaired_key(key, parent, indent);
        let gap = self.key_value_gap(key.end(), value_range.start());
        // A rewritten key drops the source slice's alignment padding, so
        // the padded separator and the borrowed round-trip both hold only
        // while the key passes through unchanged.
        let padded = is_align_colons_gap(gap) && matches!(key_text, Cow::Borrowed(_));
        let separator = if padded { gap } else { ": " };
        let key_end = end_column(&key_text, indent);
        // The value lands past the separator the text keeps, whereas it
        // fits against the canonical `": "`, the column the aligner pads
        // only where the cap allows.
        let landing = Landing {
            column: key_end + display_width(separator),
            indent,
            item: key.start(),
        };
        let value_text = self
            .replacement_for(
                &item.value,
                parent,
                key_end + CANONICAL_SEPARATOR,
                indent,
                tail,
            )
            .map_or_else(
                || self.placed_slice(&item.value, parent, landing, tail),
                Cow::Owned,
            );
        let width = settled_text_width(self.source, self.padding, &key_text, key.range())
            + 2
            + settled_text_width(self.source, self.padding, &value_text, value_range);
        let text = if padded && matches!(value_text, Cow::Borrowed(_)) {
            Cow::Borrowed(
                self.source
                    .slice(TextRange::new(key.start(), value_range.end())),
            )
        } else {
            Cow::Owned(format!("{key_text}{separator}{value_text}"))
        };
        (text, width)
    }

    /// Builds the expanded form of `expr` under `parent` as a string,
    /// recursively laying out any qualifying child collections. Every
    /// row is charged the separator a later sort leaves closing it, and
    /// a flowed row packs the entries that sort leaves in it.
    pub(super) fn expand(&self, expr: &Expr, parent: AnyNodeRef, indent: usize) -> String {
        let item_indent = item_indent(indent);
        let dict_items = expr.as_dict_expr().map(|d| &d.items);
        let node = AnyNodeRef::from(expr);
        let order = self.reorders.sorted_slots(self.source, node, parent);
        let GatheredItems {
            atomics,
            close,
            open,
            ranges,
            texts,
            widths,
        } = self.gather_items(expr, parent, item_indent);
        let last = order
            .as_ref()
            .and_then(|order| order.last())
            .map(|&index| ranges[index]);
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
                        let separator = entry_tail(last, ranges[idx], usize::from(has_more));
                        let inline = &texts[idx];
                        let row_overflows = !inline.contains('\n')
                            && item_indent + widths[idx] + separator > self.code_line_length;
                        let hung = dict_items
                            .filter(|_| row_overflows && self.wrap_dict_entries)
                            .and_then(|items| {
                                self.hang_dict_value(&items[idx], node, item_indent, separator)
                            });
                        out.push_str(&item_prefix);
                        out.push_str(hung.as_deref().unwrap_or(inline));
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
