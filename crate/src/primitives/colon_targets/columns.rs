//! The two columns a docstring entry run settles, the `(` of every
//! entry naming a parenthesized type and the `:` measured against the
//! column those `(` reach.

use ruff_python_ast::Stmt;

use crate::{
    primitives::{
        aligner,
        docstring::{body_docstring, entry_runs},
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
        let mut columns = aligner::operator_columns(source, &parens, settings).into_iter();
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

/// Returns one alignment group per entry run in the body's leading
/// docstring, each carrying a `:` row per entry anchored on its head
/// line's unbracketed `:` and a `(` row per entry naming a
/// parenthesized type. Returns an empty `Vec` when the body has no
/// leading docstring or carries no entry run. Each run is its own
/// group, so one run's widths never shift another's column.
pub(super) fn docstring_runs(source: &Source, body: &[Stmt]) -> Vec<EntryColumns> {
    let Some(lit) = body_docstring(body) else {
        return Vec::new();
    };
    entry_runs(source, lit)
        .iter()
        .map(|entries| {
            let (colons, parens) = entries
                .iter()
                .map(|entry| {
                    (
                        aligner::line_anchored_member(source, entry.colon),
                        entry
                            .paren
                            .map(|at| aligner::line_anchored_member(source, at)),
                    )
                })
                .unzip();
            EntryColumns { colons, parens }
        })
        .collect()
}
