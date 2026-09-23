//! The dict-entry half of the expand path: each entry serialized with
//! the separator it keeps, hung at `:` where its row overflows at the
//! canonical `": "`, and its value measured at the column
//! `align-colons` seats it at once the expanded rows align, so a value
//! whose one-row form overflows there expands in the same pass.

use std::borrow::Cow;

use ruff_python_ast::{AnyNodeRef, DictItem, Expr, ExprDict};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    CANONICAL_SEPARATOR, Layouter,
    classify::{is_align_colons_gap, pre_colon_padding},
};
use crate::{
    primitives::{
        INDENT_STEP,
        aligner::{self, Extent, Slot},
        inline::{display_width, end_column, opening_width, settled_text_width, spans_rows},
        layout::opener_width,
        travel::Landing,
    },
    rules::stack_adjacent_strings::concatenated_run,
};

/// One dict entry as the expand path writes it: its key text and the
/// width that key settles to, `None` and zero for a `**` unpacking,
/// the entry's text and its display width at the canonical `": "`,
/// and the offset its value starts at.
struct Entry<'a> {
    key: Option<Cow<'a, str>>,
    key_width: usize,
    text: Cow<'a, str>,
    value_start: TextSize,
    width: usize,
}

impl<'a> Layouter<'a> {
    /// Serializes a dict item as `key: value` or `**value`, paired with
    /// its display width at the canonical `": "` separator. The key
    /// routes through `repaired_key`, and the value measures and lands
    /// at `seat` where `align-colons` seats it, or otherwise fits past
    /// the key text's last row and the canonical separator and lands
    /// past the separator the text keeps. A borrowed key and value over
    /// an `align-colons`-padded gap return the source slice whole, the
    /// width still counting the canonical `": "`.
    fn entry(
        &self,
        item: &DictItem,
        parent: AnyNodeRef,
        indent: usize,
        tail: usize,
        seat: Option<usize>,
    ) -> Entry<'a> {
        let value_range = self.range_with_parens(&item.value, parent);
        let Some(key) = &item.key else {
            let value_text = self.serialize_expr(&item.value, parent, indent + 2, indent, tail);
            let width = 2 + settled_text_width(self.source, self.padding, &value_text, value_range);
            return Entry {
                key: None,
                key_width: 0,
                text: Cow::Owned(format!("**{value_text}")),
                value_start: value_range.start(),
                width,
            };
        };
        let key_text = self.repaired_key(key, parent, indent);
        let gap = self.key_value_gap(key.end(), value_range.start());
        // A rewritten key drops the source slice's alignment padding, so
        // the padded separator and the borrowed round-trip both hold only
        // while the key passes through unchanged.
        let padded = is_align_colons_gap(gap) && matches!(key_text, Cow::Borrowed(_));
        let separator = if padded { gap } else { ": " };
        let key_end = end_column(&key_text, indent);
        let landing = Landing {
            column: seat.unwrap_or(key_end + display_width(separator)),
            indent,
            item: key.start(),
        };
        let value_text = self
            .replacement_for(
                &item.value,
                parent,
                seat.unwrap_or(key_end + CANONICAL_SEPARATOR),
                indent,
                tail,
            )
            .map_or_else(
                || self.placed_slice(&item.value, parent, landing, tail),
                Cow::Owned,
            );
        let key_width = settled_text_width(self.source, self.padding, &key_text, key.range());
        let width =
            key_width + 2 + settled_text_width(self.source, self.padding, &value_text, value_range);
        let text = if padded && matches!(value_text, Cow::Borrowed(_)) {
            Cow::Borrowed(
                self.source
                    .slice(TextRange::new(key.start(), value_range.end())),
            )
        } else {
            Cow::Owned(format!("{key_text}{separator}{value_text}"))
        };
        Entry {
            key: Some(key_text),
            key_width,
            text,
            value_start: value_range.start(),
            width,
        }
    }

    /// Builds the hung two-line form of a `key: value` dict entry,
    /// breaking at `:` and emitting the value at `item_indent +
    /// INDENT_STEP` with `tail` columns closing its row. The key writes
    /// as `key_text`, the form `repaired_key` gave it, and keeps its
    /// pre-colon padding. Returns `None` for a `**value` unpacking item
    /// and for an entry with an implicitly concatenated string on either
    /// side of its `:`, which `stack-adjacent-strings` breaks in place.
    fn hang_dict_value(
        &self,
        key_text: &str,
        item: &DictItem,
        parent: AnyNodeRef,
        item_indent: usize,
        tail: usize,
    ) -> Option<String> {
        let key = item.key.as_ref()?;
        if concatenated_run(key).is_some() || concatenated_run(&item.value).is_some() {
            return None;
        }
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

    /// The `align-colons` row `entry` of `item` opens at `indent` with
    /// `tail` columns closing it: a bridge for a `**` unpacking, a break
    /// for a key spanning rows, and otherwise the key's width beside the
    /// [`Extent`] the row takes before padding, its value's opening row
    /// measured where a layout rule expands a one-row value.
    fn row(
        &self,
        entry: &Entry<'a>,
        item: &DictItem,
        indent: usize,
        tail: usize,
    ) -> Slot<(usize, Extent)> {
        let Some(key_text) = &entry.key else {
            return Slot::Bridge;
        };
        if spans_rows(key_text) {
            return Slot::Break;
        }
        let extent = if spans_rows(&entry.text) {
            Extent {
                expanded: None,
                inline: indent + opening_width(&entry.text),
            }
        } else {
            Extent {
                expanded: opener_width(self.source, &item.value, entry.value_start)
                    .map(|width| indent + entry.key_width + CANONICAL_SEPARATOR + width),
                inline: indent + entry.width + tail,
            }
        };
        Slot::Member((display_width(key_text), extent))
    }

    /// Re-serializes each of `entries` whose value `align-colons` seats
    /// under `settings` past the canonical `": "` at a column where its
    /// one-row form overflows and a layout rule expands it. The rows
    /// read in the order `order` leaves them, a run closing at a key
    /// spanning rows and ahead of a row a blank line opens, and a `**`
    /// unpacking passing a run through. Where `order` names the sort
    /// `alphabetize-siblings` reassembles the dict under, that rule drops
    /// the source's blank lines and sets one on either side of each entry
    /// spanning rows once two do, so an entry already spanning rows seats
    /// nothing and more than one value to expand expands none, since
    /// either would leave an expanded entry alone between two blank
    /// lines.
    fn seat_entries(
        &self,
        entries: &mut [Entry<'a>],
        dict: &ExprDict,
        tails: &[usize],
        indent: usize,
        order: Option<&[usize]>,
        settings: aligner::Settings,
    ) {
        let reassembled = order.is_some();
        if reassembled
            && entries
                .iter()
                .any(|entry| entry.key.is_some() && spans_rows(&entry.text))
        {
            return;
        }
        let node = AnyNodeRef::from(dict);
        let mut runs: Vec<Vec<(usize, (usize, Extent))>> = vec![Vec::new()];
        for position in 0..entries.len() {
            let index = order.map_or(position, |order| order[position]);
            let row = self.row(&entries[index], &dict.items[index], indent, tails[index]);
            if matches!(row, Slot::Break)
                || (!reassembled
                    && position > 0
                    && self
                        .source
                        .has_blank_line_before(dict.items[position].start()))
            {
                runs.push(Vec::new());
            }
            if let Slot::Member(row) = row {
                runs.last_mut()
                    .expect("a run opens before a row joins it")
                    .push((index, row));
            }
        }
        let mut seats = Vec::new();
        for run in runs {
            let (indices, rows): (Vec<usize>, Vec<(usize, Extent)>) = run.into_iter().unzip();
            let columns = aligner::written_columns(indent, &rows, settings);
            for ((index, (width, extent)), column) in indices.into_iter().zip(rows).zip(columns) {
                let padding = column - indent - width;
                if extent.expanded.is_some() && extent.inline + padding > self.code_line_length {
                    seats.push((index, column + aligner::VALUE_OFFSET));
                }
            }
        }
        if reassembled && seats.len() > 1 {
            return;
        }
        for (index, seat) in seats {
            entries[index] = self.entry(&dict.items[index], node, indent, tails[index], Some(seat));
        }
    }

    /// Collects `dict`'s entries at `indent`, each charged the separator
    /// `tail` names for its slot, the one a later sort leaves closing its
    /// row, and hung at `:` where that row overflows at the canonical
    /// `": "`. Where `align-colons` runs, each value then measures at the
    /// column that rule seats it at once the expanded rows align in the
    /// order `order` leaves them.
    pub(super) fn gather_entries(
        &self,
        dict: &ExprDict,
        indent: usize,
        order: Option<&[usize]>,
        tail: impl Fn(usize, usize, TextRange) -> usize,
    ) -> impl Iterator<Item = (Cow<'a, str>, usize, bool, TextRange)> {
        let node = AnyNodeRef::from(dict);
        let (tails, mut entries): (Vec<usize>, Vec<Entry<'a>>) = dict
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let tail = tail(i, dict.len(), item.range());
                let mut entry = self.entry(item, node, indent, tail, None);
                if self.wrap_dict_entries
                    && !spans_rows(&entry.text)
                    && indent + entry.width + tail > self.code_line_length
                    && let Some(key_text) = &entry.key
                    && let Some(hung) = self.hang_dict_value(key_text, item, node, indent, tail)
                {
                    entry.text = Cow::Owned(hung);
                }
                (tail, entry)
            })
            .unzip();
        if let Some(settings) = self.colons {
            self.seat_entries(&mut entries, dict, &tails, indent, order, settings);
        }
        entries
            .into_iter()
            .zip(dict.iter())
            .map(|(entry, item)| (entry.text, entry.width, false, item.range()))
    }
}
