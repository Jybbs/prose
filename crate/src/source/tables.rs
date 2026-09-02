//! The tables a source builds on first read, the binding table a
//! reparse carries across, moved through the weave's `SourceMap` to
//! the positions the new text carries, and the padding walk a splice
//! rebuilds over the statements it reparsed alone.

use std::{
    borrow::{Borrow, Cow},
    sync::OnceLock,
};

use ruff_diagnostics::{Edit, SourceMap};
use ruff_text_size::{Ranged, TextRange};

use super::{
    Source,
    trace::{self, Outcome},
};
use crate::{
    primitives::{
        binding::BindingAnalysis,
        colon_targets::reaches,
        edit::forward_range,
        padding::Stranding,
        reserve::{Columns, Reservations},
    },
    rule::RuleId,
};

/// The label each table reports its builds and carries under.
const BINDINGS: &str = "bindings";
const COLUMNS: &str = "columns";
const STRANDED: &str = "stranded";

impl Source {
    /// Panics where the binding table a reparse carried into this
    /// source differs from the one a fresh read builds, naming `site`
    /// in the message, and returns whether it held one to compare.
    #[cfg(test)]
    pub(crate) fn assert_carried_bindings_are_fresh(&self, site: &str) -> bool {
        let Some(carried) = self.binding_analysis.get() else {
            return false;
        };
        let fresh = BindingAnalysis::new(self.ast());
        assert!(
            **carried == fresh,
            "the binding table carried into {site} differs from a fresh build:\n{}",
            table_diff(&fresh, carried, "carried"),
        );
        true
    }

    /// Returns the binding-analysis table, built on the first read
    /// where the reparse before it carried none.
    pub fn binding_analysis(&self) -> &BindingAnalysis {
        self.binding_analysis.get_or_init(|| {
            trace::built(BINDINGS);
            Box::new(BindingAnalysis::new(self.ast()))
        })
    }

    /// Returns the columns `reservations` shifts each aligned value to,
    /// walking the tree on the first read where the reparse before it
    /// carried none. Every rule of a run measures against the same
    /// reservation and reads the walk back, whereas a read carrying a
    /// different one walks for itself.
    pub(crate) fn columns(&self, reservations: Reservations) -> Cow<'_, Columns> {
        keyed(&self.columns, COLUMNS, reservations, |reservations| {
            reservations.columns(self)
        })
    }

    /// Fills the binding table `bindings` holds where `preserves` says
    /// the splice's edits leave every binding standing, moved through
    /// `map` to the positions this text carries, so the next read finds
    /// it in place, the outcome reported under `rule`. A table an edit
    /// in `map` leaves nowhere to move, one of whose offsets that edit
    /// replaced, is left for that read to rebuild, as are the layout
    /// forecasts behind every splice.
    pub(crate) fn inherit(
        &mut self,
        bindings: OnceLock<Box<BindingAnalysis>>,
        map: &SourceMap,
        rule: RuleId,
        preserves: bool,
    ) {
        self.binding_analysis = inherited(rule, BINDINGS, preserves, bindings, |analysis| {
            analysis.forwarded(map)
        });
    }

    /// Takes the binding table out of this source's slot, leaving an
    /// empty slot behind, so a caller holds the table across a reparse
    /// that consumes the source it came from.
    pub(crate) fn take_binding_analysis(&mut self) -> OnceLock<Box<BindingAnalysis>> {
        std::mem::take(&mut self.binding_analysis)
    }

    /// Panics where the padding walk a splice rebuilt into this source
    /// differs from the one a fresh read builds, naming `site` in the
    /// message, and returns whether it held one to compare.
    #[cfg(test)]
    pub(crate) fn assert_rebuilt_padding_is_fresh(&self, site: &str) -> bool {
        let Some(rebuilt) = self.stranded_padding.get() else {
            return false;
        };
        let fresh = rebuilt.0.edits(self);
        assert!(
            rebuilt.1 == fresh,
            "the padding walk rebuilt into {site} differs from a fresh build:\n{}",
            table_diff(&fresh, &rebuilt.1, "rebuilt"),
        );
        true
    }

    /// Fills the padding walk over this text from the one `previous`
    /// holds, every entry outside `windows` moved through `map` and the
    /// entries inside them walked afresh, `windows` being the spans a
    /// splice reparsed in this text, ascending. The union equals a fresh
    /// walk while no entry outside a window changes, the property the
    /// containment test re-proves over the corpus. An entry an edit
    /// replaced sat inside a window, since every edit does, so it drops
    /// for the walk there to re-derive. The slot is left empty where
    /// `previous` holds no walk, the outcome reported under `rule`.
    pub(crate) fn rebuild_stranded_padding(
        &mut self,
        previous: OnceLock<Box<(Stranding, Vec<Edit>)>>,
        map: &SourceMap,
        windows: &[TextRange],
        rule: RuleId,
    ) {
        let held = previous.get().is_some();
        let rebuilt = previous.into_inner().map(|held| {
            let (stranding, edits) = *held;
            let mut carried: Vec<Edit> = edits
                .iter()
                .filter_map(|edit| {
                    let range = forward_range(edit.range(), map)
                        .filter(|range| !reaches(*range, windows))?;
                    Some(relocated(edit, range))
                })
                .collect();
            carried.extend(stranding.edits_within(self, windows));
            carried.sort_unstable_by_key(Ranged::start);
            Box::new((stranding, carried))
        });
        trace::carried(rule, STRANDED, Outcome::of(true, held, rebuilt.is_some()));
        self.stranded_padding = rebuilt.map_or_else(OnceLock::new, OnceLock::from);
    }

    /// Returns the edits `stranding` emits over this source, walking the
    /// tree on the first read where the reparse before it carried none.
    /// Every rule of a run measures against the same padding rule and
    /// reads the walk back, whereas a read carrying a different one
    /// walks for itself.
    pub(crate) fn stranded_padding(&self, stranding: Stranding) -> Cow<'_, [Edit]> {
        keyed(&self.stranded_padding, STRANDED, stranding, |stranding| {
            stranding.edits(self)
        })
    }
}

/// `edit` over `range` instead of its own, keeping its shape as an
/// insertion, a deletion, or a replacement.
fn relocated(edit: &Edit, range: TextRange) -> Edit {
    match edit.content() {
        Some(content) if range.is_empty() => Edit::insertion(content.to_owned(), range.start()),
        Some(content) => Edit::range_replacement(content.to_owned(), range),
        None => Edit::range_deletion(range),
    }
}

/// `slot`'s table moved through `forward` where `rule` leaves it
/// `permitted` to survive, and an empty slot where the rule declines
/// it, `slot` holds none, or the move fails, the outcome reported
/// under `table`.
fn inherited<T>(
    rule: RuleId,
    table: &'static str,
    permitted: bool,
    slot: OnceLock<Box<T>>,
    forward: impl FnOnce(T) -> Option<T>,
) -> OnceLock<Box<T>> {
    let held = slot.get().is_some();
    let moved = slot
        .into_inner()
        .filter(|_| permitted)
        .and_then(|table| forward(*table));
    trace::carried(rule, table, Outcome::of(permitted, held, moved.is_some()));
    moved
        .map(Box::new)
        .map_or_else(OnceLock::new, OnceLock::from)
}

/// The value `build` derives for `key`, read back from `slot` where it
/// already holds that key's value and built afresh otherwise, the
/// first read filling the slot and each build reported under `table`.
fn keyed<'a, K: Copy + PartialEq, B: ?Sized + ToOwned>(
    slot: &'a OnceLock<Box<(K, B::Owned)>>,
    table: &'static str,
    key: K,
    build: impl Fn(&K) -> B::Owned,
) -> Cow<'a, B> {
    let build = |key: &K| {
        trace::built(table);
        build(key)
    };
    let held = slot.get_or_init(|| Box::new((key, build(&key))));
    if held.0 == key {
        Cow::Borrowed(held.1.borrow())
    } else {
        Cow::Owned(build(&key))
    }
}

/// A unified diff of `fresh` against `held`, the message an assertion
/// over a table a reparse moved prints, `label` naming how `held`
/// arrived.
#[cfg(test)]
fn table_diff(fresh: &impl std::fmt::Debug, held: &impl std::fmt::Debug, label: &str) -> String {
    let (fresh, held) = (format!("{fresh:#?}"), format!("{held:#?}"));
    similar::TextDiff::from_lines(fresh.as_str(), held.as_str())
        .unified_diff()
        .header("fresh", label)
        .to_string()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_notebook::CellOffsets;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::{
        config::Config,
        testing::{parse, range, woven},
    };

    /// `source` reparsed over the text `edits` weave into it, the
    /// binding table it built carried forward where `preserves`.
    fn reparsed_with(mut source: Source, edits: Vec<Edit>, preserves: bool) -> Source {
        let (text, map) = woven(source.text(), edits);
        let bindings = source.take_binding_analysis();
        let mut next = source
            .reparse_carrying(text, CellOffsets::default())
            .expect("reparses");
        next.inherit(bindings, &map, RuleId::from("declaring"), preserves);
        next
    }

    /// `source` with every lazily built table filled under the default
    /// configuration.
    fn with_every_table(source: Source) -> Source {
        let config = Config::default();
        source.binding_analysis();
        source.columns(config.equals_reservations());
        source.stranded_padding(config.stranded_padding());
        source
    }

    #[test]
    fn columns_holds_the_first_reservation_and_walks_for_any_other() {
        let source = parse("x = 1\nlonger = 2\n");
        let mut disabled = Config::default();
        disabled.rules.align_equals.enabled = false;
        let aligned = Config::default().equals_reservations();
        let unaligned = disabled.equals_reservations();
        let value = TextSize::new(4);
        let written = source.column_of(value);

        let held = source.columns(aligned).column_in(&source, value);
        assert!(held > written);
        assert_eq!(source.columns(aligned).column_in(&source, value), held);
        assert_eq!(source.columns(unaligned).column_in(&source, value), written);
    }

    #[rstest]
    #[case::preserving(true)]
    #[case::declining(false)]
    fn inherit_carries_the_binding_table_the_rule_preserves_and_no_forecast(
        #[case] preserves: bool,
    ) {
        let source = with_every_table(parse("import os\nx = 1\nlonger = os\n"));
        let blank = Edit::insertion("\n".to_owned(), TextSize::new(10));

        let next = reparsed_with(source, vec![blank], preserves);

        assert_eq!(next.binding_analysis.get().is_some(), preserves);
        assert!(next.columns.get().is_none());
        assert!(next.stranded_padding.get().is_none());
        assert_eq!(
            next.assert_carried_bindings_are_fresh("the reparsed source"),
            preserves,
        );
    }

    #[test]
    fn inherit_leaves_an_unbuilt_table_unbuilt() {
        let blank = Edit::insertion("\n".to_owned(), TextSize::new(5));

        let next = reparsed_with(parse("x = 1\n"), vec![blank], true);

        assert!(next.binding_analysis.get().is_none());
    }

    #[rstest]
    #[case::a_binding_token("import os\nx = 1\nlonger = 2\n", "sys", 7, 9, false)]
    #[case::a_row_end("a = 1  # c\nbbb = 2\n", "d", 9, 10, true)]
    #[case::a_stranded_gap("x = [ 1 ]\n", "  ", 5, 6, true)]
    fn inherit_leaves_the_table_an_edit_replaced_an_offset_of(
        #[case] text: &str,
        #[case] content: &str,
        #[case] start: u32,
        #[case] end: u32,
        #[case] carried: bool,
    ) {
        let source = parse(text);
        source.binding_analysis();
        let edit = Edit::range_replacement(content.to_owned(), range(start, end));

        let next = reparsed_with(source, vec![edit], true);

        assert_eq!(
            next.assert_carried_bindings_are_fresh("the reparsed source"),
            carried,
        );
    }
}
