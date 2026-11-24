//! Where a reflow lands: whether a fold fits the budget, whether the
//! row a pair sits on overflows it, the column a pair reaches once the
//! pass's earlier edits apply and the padding, alignment, and later
//! shortening rules settle its row, and the spans an in-place shed
//! removes.

use std::iter::successors;

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{Expr, ExprCall};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use super::{
    Shedder,
    plan::{Candidate, shed_columns},
};
use crate::{
    primitives::{
        comparison::narrowing_edits_within,
        edit::{apply_inline_edits, insert_edit},
        inline::{
            display_width, run_closes_to_a_space, settled_head_column, settled_slice_width,
            settled_width, soft_wrap_runs,
        },
        layout::{is_collapsible, is_column_shaped, is_layoutable, item_indent},
        slots::starting_within,
        splice::splice_preserves_tree,
        tokens::string_run_breaks,
        walk::{Interpolations, any_over_expr_within, filter_map_within_expr},
    },
    rules::reflow_collections::ReflowCollections,
    source::Source,
};

impl Shedder<'_> {
    /// True where `candidate`'s joined row crosses the budget and a break
    /// owns its shape, read for a pair later on the row of the candidate
    /// under test.
    fn breaks(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        if let Some(&breaks) = self.breaks.borrow().get(&candidate.pair) {
            return breaks;
        }
        let breaks = self.overflows(candidate, candidates)
            && self.broken_form(candidate, candidates).is_some();
        self.breaks.borrow_mut().insert(candidate.pair, breaks);
        breaks
    }

    /// True where every line break inside `expr` falls inside an
    /// argument list a running `reflow-calls` closes or a bracketed
    /// collection literal, comprehension, or subscript
    /// [`Self::rejoins`] accepts.
    fn closes_every_break(&self, expr: &Expr) -> bool {
        let literals = filter_map_within_expr(expr, Interpolations::Skip, |inner| {
            (is_collapsible(inner)
                && inner
                    .as_tuple_expr()
                    .is_none_or(|tuple| tuple.parenthesized)
                && self.rejoins(inner))
            .then(|| inner.range())
        });
        self.holds_every_break(
            expr.range(),
            self.closing_lists(expr.range())
                .merge_by(literals, |list, literal| list.start() <= literal.start()),
        )
    }

    /// The argument lists inside `range` a running `reflow-calls` closes,
    /// ascending by start.
    fn closing_lists(&self, range: TextRange) -> impl Iterator<Item = TextRange> {
        starting_within(self.source.explodable_arguments(), range, Ranged::start)
            .filter(|_| self.one_row.closes())
            .copied()
    }

    /// The column `offset` reaches once the edits emitted so far apply
    /// and the padding and alignment rules settle its row, measured from
    /// the enclosing logical line.
    fn column_at(&self, offset: TextSize) -> usize {
        self.column_through(self.source.logical_line_start(offset))
    }

    /// The column the text through `range` reaches once the edits
    /// emitted so far apply and the padding table settles the row it
    /// closes on, `range` opening at the row or the logical line the
    /// measure reads from. Text those edits leave on the row the source
    /// writes it on starts from the column `align-equals` shifts that row
    /// to.
    fn column_through(&self, range: TextRange) -> usize {
        let placed = apply_inline_edits(self.source, range, &self.edits);
        let column = settled_head_column(self.source, self.padding, &placed, range.end(), 0);
        self.reservations
            .column_under(self.source, &self.edits, range.end(), column)
    }

    /// The columns `expr`'s one-row form takes once `reflow-calls`
    /// condenses it, `None` where no one-row form exists.
    fn condensed_width(&self, expr: &Expr) -> Option<usize> {
        self.one_row
            .condensed(self.source, expr, expr.into())
            .map(|form| {
                self.one_row
                    .text_width(self.source, self.padding, &form, expr.range())
            })
    }

    /// True where `reflow-calls` runs and an argument list it can explode
    /// or condense opens inside `call`.
    fn condenses(&self, call: TextRange) -> bool {
        self.one_row.closes()
            && starting_within(self.source.explodable_arguments(), call, Ranged::start)
                .next()
                .is_some()
    }

    /// The columns `form`, `expr`'s one-row form, takes once this pass
    /// sheds the pairs inside `expr` that `form` still carries. The form
    /// of a direct argument or a child of a rebuilt construct already
    /// leaves its pair out, each such pair taking its `(` out of the
    /// text.
    fn form_width(&self, expr: &Expr, form: &str, candidates: &[Candidate]) -> usize {
        let range = expr.range();
        let opens = |text: &str| text.matches('(').count();
        let dropped = opens(self.source.slice(range)).saturating_sub(opens(form));
        let kept = shed_columns(range, candidates).saturating_sub(2 * dropped);
        self.one_row
            .text_width(self.source, self.padding, form, range)
            .saturating_sub(kept)
    }

    /// True where an argument list inside `range` is written as a flush
    /// column, a layout `reflow-calls` leaves standing.
    fn holds_a_column_list(&self, range: TextRange) -> bool {
        starting_within(self.source.explodable_arguments(), range, Ranged::start)
            .any(|list| is_column_shaped(self.source.slice(*list)))
    }

    /// True where every line break inside `range` falls inside one of
    /// `closed`, ascending by start.
    fn holds_every_break(
        &self,
        range: TextRange,
        closed: impl IntoIterator<Item = TextRange>,
    ) -> bool {
        closed
            .into_iter()
            .try_fold(range.start(), |cursor, held| {
                (held.start() < cursor
                    || !self
                        .source
                        .contains_line_break(TextRange::new(cursor, held.start())))
                .then(|| cursor.max(held.end()))
            })
            .is_some_and(|cursor| {
                !self
                    .source
                    .contains_line_break(TextRange::new(cursor, range.end()))
            })
    }

    /// The columns `candidate`'s joined interior takes, widened by the
    /// spaces its own flush sides keep and narrowed by the columns the
    /// padding table takes off it, the columns each nested candidate
    /// sheds alongside it, and the columns its fold's
    /// [`narrowing_edits_within`] take off it. `None` for an interior no
    /// fold joins.
    fn joined_width(&self, candidate: &Candidate, candidates: &[Candidate]) -> Option<usize> {
        let bare = candidate.bare.as_ref()?;
        let narrowed: usize = if candidate.sheds {
            narrowing_edits_within(self.source, candidate.expr, self.padding)
                .iter()
                .map(|edit| edit.range().len().to_usize() - 1)
                .sum()
        } else {
            0
        };
        Some(
            settled_width(
                self.source,
                self.padding,
                candidate.inner,
                display_width(bare) + candidate.flush.spaces(),
            ) - shed_columns(candidate.inner, candidates)
                - narrowed,
        )
    }

    /// True where `reflow-collections` runs and no suppression of that
    /// rule holds `expr`.
    fn rejoins(&self, expr: &Expr) -> bool {
        self.one_row.joins_literals()
            && !self
                .source
                .suppression_map()
                .suppresses(expr, ReflowCollections::SLUG)
    }

    /// The columns `pair`'s own row carries past its closing paren once
    /// the padding table settles it, narrowed by the parentheses
    /// this pass sheds along that stretch. The measure closes at the
    /// opening paren of the first later pair on the row that breaks.
    fn tail_width(&self, pair: TextRange, candidates: &[Candidate]) -> usize {
        let mut tail = self.source.row_tail(pair.end());
        if let Some(later) = starting_within(candidates, tail, |other| other.pair.start())
            .find(|later| self.breaks(later, candidates))
        {
            tail = TextRange::new(tail.start(), later.pair.start() + TextSize::of('('));
        }
        settled_width(
            self.source,
            self.padding,
            tail,
            self.source.tail_width(tail),
        )
        .saturating_sub(shed_columns(tail, candidates))
    }

    /// True where the layout rules close or lay out every break inside
    /// `expr`, its opening row starting at `column`, once it lands across
    /// rows, which holds for
    ///
    /// - a collection literal `reflow-collections` expands, no argument
    ///   list inside it written as a flush column
    /// - a call holding on its callee chain an argument list `reflow-calls`
    ///   explodes on its count, the callee of that list fitting one row
    /// - a call with a one-row form, the callee of the first list on that
    ///   chain `reflow-calls` can explode, or the call itself where the
    ///   chain holds none, carrying no break outside an argument list that
    ///   rule closes
    ///
    /// A break a comprehension, subscript, or literal holds in a callee
    /// closes ahead of a count-exploded list alone, since `reflow-calls`
    /// measures any other list from the row its callee ends on.
    pub(super) fn closes_across_rows(
        &self,
        expr: &Expr,
        column: usize,
        candidates: &[Candidate],
    ) -> bool {
        if !self.closes_every_break(expr) {
            return false;
        }
        if !expr.is_call_expr() {
            return !self.holds_a_column_list(expr.range())
                && self
                    .one_row
                    .expands(self.source, expr, expr.into(), column, 0, self.padding);
        }
        let lists: Vec<&ExprCall> = successors(Some(expr), |&inner| match inner {
            Expr::Attribute(attribute) => Some(&*attribute.value),
            Expr::Call(call) => Some(&*call.func),
            _ => None,
        })
        .filter_map(Expr::as_call_expr)
        .filter(|call| {
            self.one_row
                .explodes_arguments(self.source, &call.arguments)
        })
        .collect();
        let callee = lists
            .last()
            .map_or_else(|| expr.range(), |call| call.func.range());
        let counted = |call: &ExprCall| {
            self.one_row.count_explodes(self.source, call)
                && self
                    .one_row
                    .condensed(self.source, &call.func, call.into())
                    .is_some_and(|form| {
                        let width = self.form_width(&call.func, &form, candidates);
                        self.one_row.fits(column + width + 1)
                    })
        };
        lists.iter().any(|&call| counted(call))
            || (self.condensed_width(expr).is_some()
                && self.holds_every_break(callee, self.closing_lists(callee)))
    }

    /// True when joining `candidate` leaves its line inside the budget
    /// once `reflow-calls` has taken its turn on the row, and false for
    /// an interior no fold joins.
    pub(super) fn fits(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let Some(width) = self.joined_width(candidate, candidates) else {
            return false;
        };
        self.shifted_column(candidate.pair.start()) + width <= self.code_line_length
    }

    /// True where `expr`, standing alone on a row one indent step past
    /// `indent`, lands across rows once the layout rules take their turn,
    /// which holds for
    ///
    /// - a collection literal written across rows, where
    ///   `reflow-collections` is off or no rejoined form of it fits that
    ///   row, and one written on one row that rule expands
    /// - a call holding an argument list `reflow-calls` can explode, where
    ///   that rule finds no one-row form fitting that row
    /// - with `reflow-calls` running, any other call whose every break
    ///   [`Self::closes_every_break`] closes and whose callee chain opens
    ///   on a literal `reflow-collections` expands on that row
    /// - with that rule off, any call spanning rows or holding a literal
    ///   `reflow-collections` expands on that row
    pub(super) fn lands_across_rows(&self, expr: &Expr, indent: usize) -> bool {
        let column = item_indent(indent);
        let spans = self.source.contains_line_break(expr.range());
        match expr {
            Expr::Call(call) if self.condenses(call.range()) => self
                .condensed_width(expr)
                .is_none_or(|width| !self.one_row.fits(column + width)),
            Expr::Call(call) if self.one_row.closes() => {
                self.closes_every_break(expr)
                    && successors(Some(expr), |&inner| match inner {
                        Expr::Attribute(attribute) => Some(&*attribute.value),
                        Expr::Call(call) => Some(&*call.func),
                        Expr::Subscript(subscript) => Some(&*subscript.value),
                        _ => None,
                    })
                    .last()
                    .is_some_and(|root| {
                        let tail = settled_slice_width(
                            self.source,
                            self.padding,
                            TextRange::new(root.end(), call.end()),
                        );
                        self.one_row
                            .rejoined(self.source, root, root.into(), column, tail)
                            .is_none()
                            && self.one_row.expands(
                                self.source,
                                root,
                                root.into(),
                                column,
                                tail,
                                self.padding,
                            )
                    })
            }
            Expr::Call(call) => {
                let width = |range| settled_slice_width(self.source, self.padding, range);
                spans
                    || any_over_expr_within(expr, Interpolations::Skip, |inner| {
                        self.one_row.expands(
                            self.source,
                            inner,
                            inner.into(),
                            column + width(TextRange::new(call.start(), inner.start())),
                            width(TextRange::new(inner.end(), call.end())),
                            self.padding,
                        )
                    })
            }
            _ if is_layoutable(expr) => {
                if spans {
                    !self.one_row.joins_literals()
                        || self
                            .one_row
                            .rejoined(self.source, expr, expr.into(), column, 0)
                            .is_none()
                } else {
                    self.one_row
                        .expands(self.source, expr, expr.into(), column, 0, self.padding)
                }
            }
            _ => false,
        }
    }

    /// True where `candidate`'s value, its pair taken off at `column`,
    /// lands on one row inside the budget once the layout rules take their
    /// turn, its row tail included. That holds for a value written on one
    /// row, a call whose every break [`Self::closes_every_break`] closes,
    /// and a collection literal, comprehension, or subscript the layout
    /// rules rejoin, each row-spanning one measured per
    /// [`Self::form_width`].
    pub(super) fn lands_on_one_row(
        &self,
        candidate: &Candidate,
        candidates: &[Candidate],
        column: usize,
    ) -> bool {
        let expr = candidate.expr;
        let tail = self.tail_width(candidate.pair, candidates);
        let row = column + candidate.flush.spaces() + tail;
        if !self.source.contains_line_break(expr.range()) {
            let width = settled_slice_width(self.source, self.padding, expr.range());
            return self.one_row.fits(row + width);
        }
        let form = match expr {
            Expr::Call(_) => self
                .closes_every_break(expr)
                .then(|| self.one_row.condensed(self.source, expr, expr.into()))
                .flatten(),
            _ if self.rejoins(expr) => {
                let shed = shed_columns(expr.range(), candidates);
                let column = row.saturating_sub(shed);
                self.one_row
                    .rejoined(self.source, expr, expr.into(), column, 0)
            }
            _ => None,
        };
        form.is_some_and(|form| {
            self.one_row
                .fits(row + self.form_width(expr, &form, candidates))
        })
    }

    /// True where `candidate`'s opened form `text`, its pair opening at
    /// `column`, leaves a statement row past the budget. That row runs
    /// through the first row of `text`, settled under the padding table,
    /// and is cut at the first argument list `reflow-calls` can explode or
    /// literal `reflow-collections` can expand on that row.
    pub(super) fn opens_past_the_budget(
        &self,
        candidate: &Candidate,
        text: &str,
        column: usize,
    ) -> bool {
        let range = candidate.expr.range();
        let row = TextRange::new(
            range.start(),
            self.source.text().line_end(range.start()).min(range.end()),
        );
        let written = settled_width(
            self.source,
            self.padding,
            row,
            display_width(text.lines().next().unwrap_or_default()),
        );
        let calls = starting_within(self.source.explodable_arguments(), range, Ranged::start)
            .filter(|_| self.one_row.closes());
        let literals = starting_within(self.source.expandable_literals(), range, Ranged::start)
            .filter(|_| self.one_row.expands_literals());
        let head = calls
            .chain(literals)
            .map(|opener| TextRange::new(range.start(), opener.start() + TextSize::of('(')))
            .filter(|head| !self.source.contains_line_break(*head))
            .map(|head| settled_slice_width(self.source, self.padding, head))
            .fold(written, usize::min);
        column + head > self.code_line_length
    }

    /// True where joining `candidate` crosses the budget on its row from
    /// the column [`Self::fits`] reads, the measure a break answers. An
    /// interior no fold joins overflows outright.
    pub(super) fn overflows(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let Some(width) = self.joined_width(candidate, candidates) else {
            return true;
        };
        let row = self.shifted_column(candidate.pair.start())
            + width
            + self.tail_width(candidate.pair, candidates);
        row > self.code_line_length
    }

    /// Emits the [`fold_edits`] closing each line-spanning whitespace run
    /// inside `candidate`'s interior, then its
    /// [`narrowing_edits_within`].
    pub(super) fn push_fold_edits(&mut self, candidate: &Candidate) {
        let narrowed = narrowing_edits_within(self.source, candidate.expr, self.padding);
        for edit in fold_edits(self.source, candidate.inner)
            .into_iter()
            .chain(narrowed)
        {
            insert_edit(&mut self.edits, edit);
        }
    }

    /// The deletion spans shedding `candidate` in place, leaving its
    /// breaks where the source wrote them: the opening paren with the
    /// horizontal whitespace around it up to a break on either side, and
    /// the span from the interior's end through the closing paren. `None`
    /// where the splice does not preserve the statement tree, the shape a
    /// pair outside any enclosing bracket takes once its boundary break
    /// loses the paren that licensed it.
    pub(super) fn shed_in_place_spans(
        &self,
        candidate: &Candidate,
    ) -> Option<(TextRange, TextRange)> {
        let Candidate { inner, pair, .. } = *candidate;
        let text = self.source.text();
        let after = &text[pair.start().to_usize() + 1..];
        let trailing = after.text_len() - after.trim_whitespace_start().text_len();
        let mut open = TextRange::at(pair.start(), TextSize::of('(') + trailing);
        // The paren gone, whitespace ahead of it would trail its row, so
        // a break directly past the span pulls that run into the span.
        if text[open.end().to_usize()..].starts_with(['\r', '\n']) {
            let before = &text[..pair.start().to_usize()];
            let leading = before.text_len() - before.trim_whitespace_end().text_len();
            open = TextRange::new(open.start() - leading, open.end());
        }
        let close = TextRange::new(inner.end(), pair.end());
        let bare = self.source.slice(TextRange::new(open.end(), close.start()));
        splice_preserves_tree(self.source, pair, &candidate.flush.padded(bare))
            .then_some((open, close))
    }

    /// The column `offset` reaches once the edits emitted so far apply
    /// and `reflow-calls` takes its turn on the row. Each outermost
    /// call ending ahead of `offset` on that row explodes while the row
    /// through `offset` still overflows the budget, dropping its closer
    /// to the row's indent and the text after it along with it.
    pub(super) fn shifted_column(&self, offset: TextSize) -> usize {
        let column = self.column_at(offset);
        if column < self.code_line_length {
            return column;
        }
        let row_start = self.source.text().line_start(offset);
        let placed = |to: TextSize| self.column_through(TextRange::new(row_start, to));
        let indent = self.source.line_indent_width(offset);
        let mut shift = 0;
        let row = placed(offset);
        for call in self
            .calls
            .iter()
            .filter(|call| row_start <= call.start() && call.end() <= offset)
        {
            if row.saturating_sub(shift) < self.code_line_length {
                break;
            }
            shift = placed(call.end()).saturating_sub(indent + 1);
        }
        column.saturating_sub(shift)
    }
}

/// The edits closing each line-spanning whitespace run inside `inner`, to
/// a single space between two tokens and to nothing against a bracket,
/// ascending by start. A run holding a break that stacks a string run
/// one part per row stays as written.
pub(super) fn fold_edits(source: &Source, inner: TextRange) -> Vec<Edit> {
    let text = source.slice(inner);
    let stacked = string_run_breaks(source, inner);
    soft_wrap_runs(text)
        .filter_map(|(begin, len)| {
            let start = inner.start() + text[..begin].text_len();
            let span = TextRange::at(start, text[begin..begin + len].text_len());
            (!stacked.iter().any(|&at| span.contains(at))).then(|| {
                if run_closes_to_a_space(text, begin, len) {
                    Edit::range_replacement(" ".to_owned(), span)
                } else {
                    Edit::range_deletion(span)
                }
            })
        })
        .collect()
}
