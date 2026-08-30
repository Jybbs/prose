//! The tables a source builds on first read and carries across a
//! reparse, each moved through the weave's `SourceMap` to the
//! positions the new text carries.

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
        edit::forward_edits,
        padding::Stranding,
        reserve::{Columns, Reservations},
    },
    rule::{Rule, RuleId},
};

/// The label each table reports its builds and carries under.
const BINDINGS: &str = "bindings";
const COLUMNS: &str = "columns";
const STRANDED: &str = "stranded";

impl Source {
    /// Panics where a table a reparse carried into this source differs
    /// from the one a fresh read builds, naming `site` in the message,
    /// and returns the name of each table it compared.
    #[cfg(test)]
    pub(crate) fn assert_carried_tables_are_fresh(&self, site: &str) -> Vec<&'static str> {
        let mut compared = Vec::new();
        if let Some(carried) = self.binding_analysis.get() {
            let fresh = BindingAnalysis::new(self.ast());
            assert!(
                **carried == fresh,
                "the binding analysis table carried into {site} differs from a fresh build:\n{}",
                {
                    let (fresh, carried) = (format!("{fresh:#?}"), format!("{carried:#?}"));
                    similar::TextDiff::from_lines(fresh.as_str(), carried.as_str())
                        .unified_diff()
                        .header("fresh", "carried")
                        .to_string()
                },
            );
            compared.push("binding analysis");
        }
        compared.extend(keyed_fresh(
            &self.columns,
            |reservations| reservations.columns(self),
            "columns",
            site,
        ));
        compared.extend(keyed_fresh(
            &self.stranded_padding,
            |stranding| stranding.edits(self),
            "stranded padding",
            site,
        ));
        compared
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

    /// Fills the tables `previous` built and `rule` declares its edits
    /// leave standing, each moved through `map` to the positions this
    /// text carries, so the next read finds the table in place. A table
    /// an edit in `map` leaves nowhere to move, one of whose offsets
    /// that edit replaced, is left for that read to rebuild.
    pub(crate) fn inherit(&mut self, previous: Source, map: &SourceMap, rule: &dyn Rule) {
        let (id, preserves) = (rule.id(), rule.preserves());
        self.binding_analysis = inherited(
            id,
            BINDINGS,
            preserves.bindings(),
            previous.binding_analysis,
            |analysis| analysis.forwarded(map),
        );
        self.columns = inherited(
            id,
            COLUMNS,
            preserves.rows(),
            previous.columns,
            |(key, columns)| Some((key, columns.forwarded(map)?)),
        );
        self.stranded_padding = inherited(
            id,
            STRANDED,
            preserves.rows(),
            previous.stranded_padding,
            |(key, edits)| Some((key, forward_edits(edits, map)?)),
        );
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
    let moved = permitted
        .then(|| slot.into_inner())
        .flatten()
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

/// Panics where the table `slot` holds differs from the one `rebuild`
/// derives for its key, naming `label` and `site`, and returns the
/// label where the slot held one to compare.
#[cfg(test)]
fn keyed_fresh<K: Copy, V: std::fmt::Debug + PartialEq>(
    slot: &OnceLock<Box<(K, V)>>,
    rebuild: impl FnOnce(K) -> V,
    label: &'static str,
    site: &str,
) -> Option<&'static str> {
    let (key, held) = &**slot.get()?;
    assert_eq!(
        *held,
        rebuild(*key),
        "the {label} table carried into {site} differs from a fresh build",
    );
    Some(label)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_notebook::CellOffsets;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::{
        config::Config,
        rule::Preserves,
        testing::{parse, range, with_every_table, woven},
    };

    /// A rule editing nothing and declaring its edits preserve `.0`.
    struct Declaring(Preserves);

    impl Rule for Declaring {
        fn id(&self) -> RuleId {
            RuleId::from("declaring")
        }

        fn message(&self) -> &'static str {
            "declaring test rule"
        }

        fn preserves(&self) -> Preserves {
            self.0
        }
    }

    /// `source` reparsed over the text `edits` weave into it, with
    /// every table it built carried forward under `preserves`.
    fn reparsed_with(source: Source, edits: Vec<Edit>, preserves: Preserves) -> Source {
        let (text, map) = woven(source.text(), edits);
        let mut next = source
            .reparse_carrying(text, CellOffsets::default())
            .expect("reparses");
        next.inherit(source, &map, &Declaring(preserves));
        next
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
    #[case::all(Preserves::All, true, true)]
    #[case::bindings_alone(Preserves::Bindings, true, false)]
    #[case::nothing(Preserves::Nothing, false, false)]
    fn inherit_carries_each_table_the_rule_preserves(
        #[case] preserves: Preserves,
        #[case] bindings: bool,
        #[case] rows: bool,
    ) {
        let source = with_every_table(parse("import os\nx = 1\nlonger = os\n"), &Config::default());
        let blank = Edit::insertion("\n".to_owned(), TextSize::new(10));

        let next = reparsed_with(source, vec![blank], preserves);

        assert_eq!(next.binding_analysis.get().is_some(), bindings);
        assert_eq!(next.columns.get().is_some(), rows);
        assert_eq!(next.stranded_padding.get().is_some(), rows);
        next.assert_carried_tables_are_fresh("the reparsed source");
    }

    #[test]
    fn inherit_leaves_an_unbuilt_table_unbuilt() {
        let blank = Edit::insertion("\n".to_owned(), TextSize::new(5));

        let next = reparsed_with(parse("x = 1\n"), vec![blank], Preserves::All);

        assert!(next.binding_analysis.get().is_none());
        assert!(next.columns.get().is_none());
        assert!(next.stranded_padding.get().is_none());
    }

    #[rstest]
    #[case::a_binding_token("import os\nx = 1\nlonger = 2\n", "sys", 7, 9, false, true, true)]
    #[case::a_row_end("a = 1  # c\nbbb = 2\n", "d", 9, 10, true, false, true)]
    #[case::a_stranded_gap("x = [ 1 ]\n", "  ", 5, 6, true, true, false)]
    fn inherit_leaves_the_table_an_edit_replaced_an_offset_of(
        #[case] text: &str,
        #[case] content: &str,
        #[case] start: u32,
        #[case] end: u32,
        #[case] bindings: bool,
        #[case] columns: bool,
        #[case] stranded: bool,
    ) {
        let source = with_every_table(parse(text), &Config::default());
        let edit = Edit::range_replacement(content.to_owned(), range(start, end));

        let next = reparsed_with(source, vec![edit], Preserves::All);

        assert_eq!(next.binding_analysis.get().is_some(), bindings);
        assert_eq!(next.columns.get().is_some(), columns);
        assert_eq!(next.stranded_padding.get().is_some(), stranded);
        next.assert_carried_tables_are_fresh("the reparsed source");
    }
}
