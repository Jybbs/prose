//! Measures each reserved value the way the layout rules read it before
//! they join it onto one row or break it open, so a statement run cuts
//! where those rules end it, beside the seatings and the `align-colons`
//! settings its runs form under.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use super::visit::Run;
use crate::{
    primitives::{
        aligner,
        inline::{display_width, settled_row_tail, settled_slice_width},
        layout::{is_fractured, opener_width},
        one_row,
        orderer::Seatings,
    },
    source::Source,
};

/// The terms a reservation measures each value under over one source,
/// `one_row` resolving calls against the module's targets and reading
/// through the forecast f-string rewrites, `padding` the deletions
/// `strip-stranded-padding` makes merged with those rewrites,
/// `seatings` the rows of each body a reorder rule seats other than as
/// written, keyed by the start of its first statement, and `colons` the
/// settings `align-colons` pads an annotated row's `:` under, `None`
/// where that rule is off.
pub(super) struct Measure<'a> {
    pub(super) colons: Option<aligner::Settings>,
    pub(super) one_row: one_row::Settings<'a>,
    pub(super) padding: &'a [Edit],
    pub(super) seatings: &'a Seatings,
    pub(super) source: &'a Source,
}

impl Measure<'_> {
    /// Measures the width `expr` over `range` settles to on one row, its
    /// written width where it holds one row already and otherwise the width
    /// of the form a layout rule joins it back to. That form is a fractured
    /// call no count trigger explodes, rejoined through its one-row argument
    /// list, or a collection whose joined form fits the budget, and any other
    /// value spanning rows returns `None`.
    fn one_row_width(&self, expr: &Expr, parent: AnyNodeRef, range: TextRange) -> Option<usize> {
        if !self.source.contains_line_break(range) {
            return Some(settled_slice_width(self.source, self.padding, range));
        }
        let Expr::Call(call) = expr else {
            let form =
                self.one_row
                    .rejoined(self.source, expr, parent, 0, self.tail(range.end()))?;
            return Some(
                self.one_row
                    .text_width(self.source, self.padding, &form, range),
            );
        };
        let arguments = &call.arguments;
        let callee = TextRange::new(range.start(), arguments.start());
        if self.source.contains_line_break(callee)
            || !is_fractured(self.source, arguments.range())
            || self.one_row.count_explodes(self.source, call)
        {
            return None;
        }
        let form = self.one_row.arguments_form(self.source, arguments)?;
        Some(
            settled_slice_width(self.source, self.padding, callee)
                + self
                    .one_row
                    .text_width(self.source, self.padding, &form, arguments.range()),
        )
    }

    /// Measures the width the code past `end` takes on its row once the
    /// padding settles, a trailing comment closing the measure.
    fn tail(&self, end: TextSize) -> usize {
        settled_row_tail(self.source, self.padding, end)
    }

    /// Measures the widths a statement row whose value `expr` opens at
    /// `column` reaches on one row and broken open, returning `None` where no
    /// layout rule breaks `expr` open or, for a value spanning rows, joins it
    /// back onto one row.
    pub(super) fn breakable(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
    ) -> Option<aligner::Breakable> {
        let range = self.source.paren_aware_range(expr.into(), parent);
        let opener = opener_width(self.source, &self.one_row, expr, range.start())?;
        Some(aligner::Breakable {
            code: column + self.one_row_width(expr, parent, range)? + self.tail(range.end()),
            opener: column + opener,
        })
    }

    /// Computes the shift `align-colons` gives each annotated row of `runs`,
    /// keyed by the row's line start and empty where that rule is off, each
    /// row held where its colon run continues below it. Across each run that
    /// aligns, a row shifts by the padding its `:` takes past its name and the
    /// one space its post-colon gap settles to, less those two gaps as
    /// written.
    pub(super) fn colon_shifts(
        &self,
        runs: &[Run],
        widenings: &aligner::Widenings,
    ) -> FxHashMap<TextSize, aligner::Statement> {
        let Some(settings) = self.colons else {
            return FxHashMap::default();
        };
        let written = |gap: TextRange| display_width(self.source.slice(gap)).cast_signed();
        runs.iter()
            .filter(|run| aligner::is_alignment_candidate(&run.members))
            .flat_map(|run| {
                let columns =
                    aligner::operator_columns(self.source, &run.members, settings, widenings, &[]);
                let last = run.members.len() - 1;
                run.members
                    .iter()
                    .zip(columns)
                    .enumerate()
                    .map(move |(index, (member, column))| {
                        let padding =
                            (column - member.baseline - member.settled_width).cast_signed();
                        let value_gap = member
                            .rewritten_value_gap(self.source)
                            .map_or(0, |gap| 1 - written(gap));
                        (
                            member.line_start,
                            aligner::Statement {
                                breaks: None,
                                holds: index < last,
                                shift: padding - written(member.gap) + value_gap,
                            },
                        )
                    })
            })
            .collect()
    }

    /// Measures the width the row of `expr` reaches at `column` once a layout
    /// rule joins a value spanning rows onto it, returning `None` for a value
    /// on one row or one no rule joins. A statement's value, where `statement`
    /// is set, reads at its one-row width wherever it can also break open,
    /// leaving [`aligner::breaking_columns`] to decide between the two, and
    /// any other value only where its joined form fits at `column`.
    pub(super) fn joined(
        &self,
        expr: &Expr,
        parent: AnyNodeRef,
        column: usize,
        statement: bool,
    ) -> Option<usize> {
        let range = self.source.paren_aware_range(expr.into(), parent);
        if !self.source.contains_line_break(range) {
            return None;
        }
        if statement && let Some(row) = self.breakable(expr, parent, column) {
            return Some(row.code);
        }
        let tail = self.source.row_tail_width(range.end());
        let form = self
            .one_row
            .rejoined(self.source, expr, parent, column, tail)?;
        Some(column + display_width(&form) + tail)
    }

    /// Reports whether `stmt` spans rows only through a value a layout rule
    /// joins onto one row or breaks open, the choice
    /// [`aligner::breaking_columns`] makes.
    pub(super) fn joins(&self, stmt: &Stmt) -> bool {
        assigned_value(stmt).is_some_and(|value| {
            self.source.contains_line_break(value.range())
                && self.breakable(value, stmt.into(), 0).is_some()
        })
    }

    /// Looks up the rows a reorder rule seats `body` in, returning `None`
    /// where every rule leaves it as written.
    pub(super) fn seating(&self, body: &[Stmt]) -> Option<&[(usize, bool)]> {
        let first = body.first()?;
        self.seatings.get(&first.start()).map(Vec::as_slice)
    }
}

/// Returns the value an assignment, augmented assignment, or initialized
/// annotated assignment binds, or `None` for any other statement.
pub(super) fn assigned_value(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Assign(a) => Some(&a.value),
        Stmt::AugAssign(a) => Some(&a.value),
        Stmt::AnnAssign(a) => a.value.as_deref(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{
        config::Config,
        testing::{first_value, parse},
    };

    /// Returns the widths [`Measure::breakable`] measures for the first
    /// statement of `src` under the default configuration, its value opening
    /// at column 4.
    fn breakable(src: &str) -> Option<(usize, usize)> {
        let source = parse(src);
        let stmt = &source.ast().body[0];
        Config::default()
            .equals_reservations()
            .measured(&source, |measure| {
                measure
                    .breakable(first_value(&source), stmt.into(), 4)
                    .map(|row| (row.code, row.opener))
            })
    }

    #[rstest]
    #[case::one_row_call("x = frob(a, b)\n", Some((14, 9)))]
    #[case::fractured_call("x = frob(a,\n         b)\n", Some((14, 9)))]
    #[case::fractured_list("x = [a,\n     b]\n", Some((10, 5)))]
    #[case::column_shaped_call("x = frob(\n    a,\n    b\n)\n", None)]
    #[case::trailing_comment("x = frob(a, b)  # note\n", Some((14, 9)))]
    #[case::name("x = value\n", None)]
    fn breakable_reads_a_row_on_one_line_and_broken_open(
        #[case] src: &str,
        #[case] expected: Option<(usize, usize)>,
    ) {
        assert_eq!(breakable(src), expected);
    }

    #[rstest]
    #[case::fractured_call("x = frob(a,\n         b)\n", true)]
    #[case::column_shaped_call("x = frob(\n    a,\n    b\n)\n", false)]
    #[case::one_row_call("x = frob(a, b)\n", false)]
    #[case::bare_expression("frob(a,\n     b)\n", false)]
    fn joins_names_a_statement_spanning_rows_through_a_breakable_value(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let source = parse(src);
        let joins = Config::default()
            .equals_reservations()
            .measured(&source, |measure| measure.joins(&source.ast().body[0]));
        assert_eq!(joins, expected);
    }
}
