//! The dict-entry half of the expand path: each entry serialized with
//! the separator it keeps, hung at `:` where its row overflows at the
//! canonical `": "`, and its value measured at the column
//! `align-colons` seats it at once the expanded rows align, so a value
//! whose one-row form overflows there expands in the same pass wherever
//! that leaves its run in fewer columns, or in as many with fewer rows
//! standing alone.

use std::borrow::Cow;

use ruff_python_ast::{AnyNodeRef, DictItem, Expr, ExprDict, token::TokenKind};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{
    CANONICAL_SEPARATOR, Layouter,
    classify::{is_align_colons_gap, pre_colon_padding},
};
use crate::{
    primitives::{
        INDENT_STEP,
        aligner::{self, Extent, Slot},
        colon_targets::dict_entry_slot,
        inline::{display_width, end_column, opening_width, settled_text_width, spans_rows},
        layout::opener_width,
        tokens::code_token_before,
        travel::Landing,
    },
    rules::{
        align_colons::AlignColons, alphabetize_siblings::sets_dividers,
        stack_adjacent_strings::concatenated_run,
    },
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
    /// Builds the [`Entry`] for a dict item written as `key: value` or
    /// `**value`, its width counted at the canonical `": "` separator.
    /// The value is measured and lands at `seat` where one is given, and
    /// otherwise is measured past the key's last row and the canonical
    /// separator while landing past the separator the text keeps. A
    /// borrowed key and value over an `align-colons`-padded gap return
    /// the source slice whole.
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
        let width = key_width
            + CANONICAL_SEPARATOR
            + settled_text_width(self.source, self.padding, &value_text, value_range);
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

    /// Returns the offset `item` opens at, which for a `**` unpacking is
    /// its `**` token, whereas ruff's own range opens at the value.
    fn entry_start(&self, item: &DictItem, parent: AnyNodeRef) -> TextSize {
        if item.key.is_some() {
            return item.start();
        }
        let value_start = self.range_with_parens(&item.value, parent).start();
        code_token_before(self.source.tokens(), value_start)
            .filter(|token| token.kind() == TokenKind::DoubleStar)
            .map_or(item.start(), Ranged::start)
    }

    /// Builds the hung two-line form of a `key: value` dict entry,
    /// breaking at `:` and emitting the value at `item_indent +
    /// INDENT_STEP` with `tail` columns closing its row. The key is
    /// written as `key_text`, the form `repaired_key` returns, and keeps
    /// its pre-colon padding. Returns `None` for a `**value` unpacking item
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

    /// Returns the `align-colons` slot for `entry`'s row, which opens at
    /// `indent` with `tail` columns closing it: a bridge for a `**`
    /// unpacking or for a row a skip holds for `align-colons`, a break for
    /// a key spanning rows, and otherwise the key's settled width beside
    /// the row's [`Extent`], which measures the value's opening row where
    /// a layout rule can expand a one-row value.
    fn row(
        &self,
        entry: &Entry<'a>,
        item: &DictItem,
        indent: usize,
        tail: usize,
    ) -> Slot<(usize, Extent)> {
        dict_entry_slot(self.source, AlignColons::SLUG, item, || {
            let key_text = entry
                .key
                .as_deref()
                .expect("a keyed item carries its key text");
            if spans_rows(key_text) {
                return Slot::Break;
            }
            let extent = if spans_rows(&entry.text) {
                Extent {
                    expanded: None,
                    inline: indent
                        + entry.key_width
                        + opening_width(entry.text[key_text.len()..].trim_start_matches(' ')),
                }
            } else {
                Extent {
                    expanded: opener_width(
                        self.source,
                        &item.value,
                        entry.value_start,
                        self.one_row.closes(),
                    )
                    .map(|width| indent + entry.key_width + CANONICAL_SEPARATOR + width),
                    inline: indent + entry.width + tail,
                }
            };
            Slot::Member((entry.key_width, extent))
        })
    }

    /// Re-serializes each of `entries` whose one-row value overflows at
    /// the column `align-colons` seats it at under `settings`, where a
    /// layout rule can expand that value and doing so leaves the run in
    /// fewer groups, or in as many with fewer groups of one, reading rows
    /// in the order `order` leaves them. A key spanning rows or, without
    /// a sort, a blank line closes a run, and a `**` unpacking passes
    /// through one. Under a sort, nothing is re-serialized where
    /// [`sets_dividers`] would hold.
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
                        .has_blank_line_before(self.entry_start(&dict.items[index], node)))
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
            let rows: Vec<(usize, Extent)> = run.iter().map(|&(_, row)| row).collect();
            let columns = aligner::written_columns(indent, &rows, settings);
            let overflowing: Vec<(usize, usize)> = run
                .iter()
                .zip(columns)
                .filter_map(|(&(index, (width, extent)), column)| {
                    let padding = column - indent - width;
                    (extent.expanded.is_some() && extent.inline + padding > self.code_line_length)
                        .then_some((index, column + aligner::VALUE_OFFSET))
                })
                .collect();
            if overflowing.is_empty() {
                continue;
            }
            let one_row: Vec<(usize, Extent)> = rows
                .iter()
                .map(|&(width, extent)| {
                    (
                        width,
                        Extent {
                            expanded: None,
                            ..extent
                        },
                    )
                })
                .collect();
            if aligner::written_groups(indent, &rows, settings)
                < aligner::written_groups(indent, &one_row, settings)
            {
                seats.extend(overflowing);
            }
        }
        if reassembled
            && sets_dividers(entries.iter().enumerate().map(|(index, entry)| {
                entry.key.is_some()
                    && (spans_rows(&entry.text) || seats.iter().any(|&(seat, _)| seat == index))
            }))
        {
            return;
        }
        for (index, seat) in seats {
            entries[index] = self.entry(&dict.items[index], node, indent, tails[index], Some(seat));
        }
    }

    /// Collects `dict`'s entries at `indent`, charging each the separator
    /// `tail` names for its slot and hanging it at `:` where that row
    /// overflows at the canonical `": "`. Where `align-colons` is on, each
    /// value is then measured at the column that rule seats it at once the
    /// expanded rows align in the order `order` leaves them.
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
            .map(move |(entry, item)| {
                let range = TextRange::new(self.entry_start(item, node), item.end());
                (entry.text, entry.width, false, range)
            })
    }
}
