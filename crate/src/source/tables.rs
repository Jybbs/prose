//! The tables a source builds on first read, and the binding table a
//! reparse carries across, moved through the weave's `SourceMap` to
//! the positions the new text carries.

use std::{
    borrow::{Borrow, Cow},
    sync::OnceLock,
};

use ruff_diagnostics::{Edit, SourceMap};

use super::{
    Source,
    trace::{self, Outcome},
};
use crate::{
    primitives::{
        binding::BindingAnalysis,
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
            {
                let (fresh, carried) = (format!("{fresh:#?}"), format!("{carried:#?}"));
                similar::TextDiff::from_lines(fresh.as_str(), carried.as_str())
                    .unified_diff()
                    .header("fresh", "carried")
                    .to_string()
            },
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
