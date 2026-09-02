//! The two columns a docstring entry run settles, the `(` of every
//! entry naming a parenthesized type and the `:` measured against the
//! column those `(` reach.

use ruff_text_size::{Ranged, TextRange};

use crate::{
    primitives::{
        aligner,
        docstring::{entry_runs, walk_docstrings},
        range::overlaps,
    },
    source::Source,
};

/// The alignment rows of one docstring entry run, a `:` row per entry
/// and a `(` row per entry naming a parenthesized type.
pub(crate) struct EntryColumns {
    colons: Vec<aligner::Member>,
    parens: Vec<Option<aligner::Member>>,
}

impl EntryColumns {
    /// The `(` rows, one per entry naming a type.
    fn parens(&self) -> Vec<aligner::Member> {
        self.parens.iter().flatten().copied().collect()
    }

    /// The `:` rows, one per entry in the run.
    pub(super) fn colons(&self) -> &[aligner::Member] {
        &self.colons
    }

    /// The run's `(` rows and its `:` rows measured against the column
    /// those `(` rows settle on under `settings`, which is their shared
    /// column where they form an alignment candidate and each row's own
    /// buffer otherwise. A `:` row whose entry names no type keeps its
    /// written width.
    pub(crate) fn settled_columns(
        &self,
        source: &Source,
        settings: aligner::Settings,
    ) -> (Vec<aligner::Member>, Vec<aligner::Member>) {
        let parens = self.parens();
        let mut columns = aligner::operator_columns(
            source,
            &parens,
            settings,
            &aligner::Widenings::default(),
            &[],
        )
        .into_iter();
        let colons = self
            .colons
            .iter()
            .zip(&self.parens)
            .map(|(colon, paren)| {
                let Some(paren) = paren else {
                    return *colon;
                };
                let column = columns.next().expect("one column per type-naming row");
                let settled = colon.width + column - source.column_of(paren.gap.end());
                colon.with_settled_width(settled)
            })
            .collect();
        (parens, colons)
    }
}

/// Returns one alignment group per entry run in every docstring
/// `source` carries, each holding a `:` row per entry anchored on its
/// head line's unbracketed `:` and a `(` row per entry naming a
/// parenthesized type. Each run is its own group, so one run's widths
/// never shift another's column.
pub(super) fn docstring_runs_within(source: &Source, windows: &[TextRange]) -> Vec<EntryColumns> {
    let mut literals = Vec::new();
    walk_docstrings(source, |_, lit| {
        if overlaps(lit.range(), windows) {
            literals.push(lit);
        }
    });
    literals
        .into_iter()
        .flat_map(|lit| entry_runs(source, lit))
        .map(|entries| {
            let (colons, parens) = entries
                .iter()
                .map(|entry| {
                    (
                        aligner::line_anchored_member(source, entry.colon),
                        entry
                            .column_anchor(source)
                            .map(|at| aligner::line_anchored_member(source, at)),
                    )
                })
                .unzip();
            EntryColumns { colons, parens }
        })
        .collect()
}
