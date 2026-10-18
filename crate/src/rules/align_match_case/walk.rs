//! Walks match statements collecting the case arms that align.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    MatchCase, Stmt, StmtMatch,
    helpers::is_compound_statement,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::*;
use crate::primitives::tokens::{is_closer, is_opener, tokens_within};

pub(super) struct Visitor<'a> {
    pub(super) code_line_length: usize,
    pub(super) reservations: &'a reserve::Reservations,
    pub(super) settling: Settling,
    pub(super) stranding: Stranding,
    pub(super) walker: aligner::AlignWalker<'a>,
    pub(super) widenings: OnceCell<aligner::Widenings>,
}

impl Visitor<'_> {
    /// Emits the alignment and collapse edits for `arms` as one fix
    /// group. Where an arm's folded line at the run's column would exceed
    /// `code_line_length`, records [`Self::unfold_edits`] for the first
    /// such arm instead, splitting it only where its body holds one row,
    /// and emits the arms on either side as runs of their own.
    fn emit_run(&mut self, arms: &[Arm]) {
        match self.first_overflow(arms) {
            None => {
                let members: Vec<_> = arms.iter().map(|arm| arm.member).collect();
                self.walker
                    .emit_group_with_gaps(&members, arms.iter().map(|arm| arm.collapse));
            }
            Some(i) => {
                let arm = &arms[i];
                self.emit_run(&arms[..i]);
                self.walker.push_group(
                    self.unfold_edits(arm.member, (!arm.hangs).then_some(arm.collapse)),
                );
                self.emit_run(&arms[i + 1..]);
            }
        }
    }

    /// Returns the index of the first arm in `arms` whose folded line
    /// would exceed `code_line_length` with its `:` padded to the column
    /// `operator_columns` resolves for it, or `None` where every arm fits.
    fn first_overflow(&self, arms: &[Arm]) -> Option<usize> {
        let members: Vec<_> = arms.iter().map(|arm| arm.member).collect();
        self.walker
            .operator_columns(&members)
            .into_iter()
            .zip(arms)
            .position(|(column, arm)| {
                column + aligner::VALUE_OFFSET + arm.body_width > self.code_line_length
            })
    }

    /// Returns the display width `body` takes from its start through the
    /// end of the row it opens on once folded, reading the delimiter
    /// padding `strip-stranded-padding` removes, the gaps `align-equals`
    /// seats, and a trailing comment at the widths their rules settle them
    /// to, and any other tail as written. A body hanging from a bracket
    /// that ends its first row is measured through that bracket.
    fn folded_width(&self, body: &Stmt, member: aligner::Member) -> usize {
        let source = self.walker.source;
        let slack = aligner::comment_slack(source, body.start(), member.gap, self.settling);
        let widening = self
            .widenings
            .get_or_init(|| self.reservations.widenings(source))
            .on_line(source.text().line_start(body.start()));
        let row = source.row_tail(body.start());
        settled_slice_width(source, &source.stranded_padding(self.stranding), row)
            .saturating_add_signed(widening - slack)
    }

    /// Emits collapse-and-align edits for one match by walking each
    /// case through `qualify_case` and dispatching on its outcome.
    fn process_match(&mut self, m: &StmtMatch) {
        let mut run = Vec::new();
        for case in &m.cases {
            if self.walker.is_held(case.start()) {
                continue;
            }
            match self.qualify_case(case) {
                CaseOutcome::Align(arm) => run.push(arm),
                CaseOutcome::Closes(arm) => {
                    run.push(arm);
                    self.emit_run(&std::mem::take(&mut run));
                }
                CaseOutcome::Disqualify(member) => {
                    self.emit_run(&std::mem::take(&mut run));
                    let edits =
                        member.map_or_else(Vec::new, |member| self.unfold_edits(member, None));
                    self.walker.push_group(edits);
                }
                CaseOutcome::Overflow(arm) => {
                    self.emit_run(&std::mem::take(&mut run));
                    self.walker
                        .push_group(self.unfold_edits(arm.member, Some(arm.collapse)));
                }
            }
        }
        self.emit_run(&run);
    }

    /// Classifies one arm into its `CaseOutcome`, returning `Disqualify`
    /// for a pattern spanning lines, a multi-statement or compound body, a
    /// multi-line body other than one hanging from its `case` row, or a
    /// comment in the `:`-to-body gap. An arm whose folded line alone
    /// exceeds `code_line_length` returns `Overflow`, or `Disqualify` where
    /// its body hangs, and a hanging arm that fits returns `Closes`.
    fn qualify_case(&self, case: &MatchCase) -> CaseOutcome {
        let source = self.walker.source;
        let Some(member) = colon_targets::match_case(source, case, self.stranding) else {
            return CaseOutcome::Disqualify(None);
        };
        let [body_first] = case.body.as_slice() else {
            return CaseOutcome::Disqualify(Some(member));
        };
        let collapse = TextRange::new(member.gap.end() + TextSize::of(':'), body_first.start());
        let hangs = source.contains_line_break(body_first);
        if is_compound_statement(body_first)
            || source.intersects_comment(collapse)
            || (hangs
                && (source.contains_line_break(collapse)
                    || !hangs_from_its_row(source, body_first, case.start())))
        {
            return CaseOutcome::Disqualify(Some(member));
        }
        let arm = Arm {
            body_width: self.folded_width(body_first, member),
            collapse,
            hangs,
            member,
        };
        match (self.first_overflow(std::slice::from_ref(&arm)), hangs) {
            (Some(_), true) => CaseOutcome::Disqualify(Some(member)),
            (Some(_), false) => CaseOutcome::Overflow(arm),
            (None, true) => CaseOutcome::Closes(arm),
            (None, false) => CaseOutcome::Align(arm),
        }
    }

    /// Returns the edits that keep an arm out of every run, removing the
    /// padding before its `:` and replacing a `collapse` gap that holds no
    /// line break with a newline and the body indent.
    fn unfold_edits(&self, member: aligner::Member, collapse: Option<TextRange>) -> Vec<Edit> {
        let source = self.walker.source;
        let split = collapse
            .filter(|collapse| !source.contains_line_break(*collapse))
            .map(|collapse| {
                let body_indent = item_indent(source.line_indent_width(member.line_start));
                let replacement = format!("{}{}", source.newline_str(), " ".repeat(body_indent));
                Edit::range_replacement(replacement, collapse)
            });
        aligner::space_padding_edit(source, member.gap, 0)
            .into_iter()
            .chain(split)
            .collect()
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Match(m) = stmt {
            self.process_match(m);
        }
        walk_stmt(self, stmt);
    }
}

/// True where `body`, folded onto the row the `case` at `case_start`
/// opens, spans rows only by hanging from a bracket that ends that row,
/// with no comment after the bracket and the matching closer opening the
/// body's last row at the `case` line's indent.
fn hangs_from_its_row(source: &Source, body: &Stmt, case_start: TextSize) -> bool {
    let Some(opener) = tokens_within(source, source.row_tail(body.start()))
        .last()
        .filter(|token| is_opener(token.kind()))
    else {
        return false;
    };
    let mut depth = 0_usize;
    tokens_within(source, TextRange::new(opener.start(), body.end()))
        .find(|token| {
            if is_opener(token.kind()) {
                depth += 1;
            } else if is_closer(token.kind()) {
                depth = depth.saturating_sub(1);
            }
            depth == 0
        })
        .is_some_and(|closer| {
            let row = source.text().line_start(closer.start());
            source.same_line(closer.start(), body.end())
                && source
                    .slice(TextRange::new(row, closer.start()))
                    .trim()
                    .is_empty()
                && source.line_indent_width(row) == source.line_indent_width(case_start)
        })
}

/// An arm whose single-statement body folds onto its `case` line.
struct Arm {
    /// The display width [`Visitor::folded_width`] measures for the
    /// body's first row once its padding, `=` gaps, and trailing comment
    /// settle.
    body_width: usize,
    /// The `:`-to-body gap the fold collapses to one space.
    collapse: TextRange,
    /// Whether the body sits folded already and hangs its later rows from
    /// a bracket ending the `case` row.
    hangs: bool,
    /// The row whose `:` aligns with the run.
    member: aligner::Member,
}

/// Outcome of qualifying one `case` arm. `Align` enrolls the arm in
/// the active run, and `Closes` enrolls a hanging arm as the run's last
/// member and ends the run. `Disqualify` ends the run and removes the
/// padding before the arm's `:` where the `:` sits on the line the
/// pattern opens on. `Overflow` ends the run and leaves the arm
/// multi-line through [`Visitor::unfold_edits`], splitting it where it
/// sits folded.
enum CaseOutcome {
    Align(Arm),
    Closes(Arm),
    Disqualify(Option<aligner::Member>),
    Overflow(Arm),
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case::explode_layout("match x:\n    case 1: y = f(\n        a\n    )\n", true)]
    #[case::closer_carrying_a_tail("match x:\n    case 1: y = f(\n        a\n    ).b\n", true)]
    #[case::comment_after_the_opener(
        "match x:\n    case 1: y = f(  # note\n        a\n    )\n",
        false
    )]
    #[case::row_ending_past_the_opener("match x:\n    case 1: y = f(a,\n        b\n    )\n", false)]
    #[case::closer_trailing_its_argument("match x:\n    case 1: y = f(\n        a)\n", false)]
    #[case::closer_past_the_case_indent(
        "match x:\n    case 1: y = f(\n        a\n        )\n",
        false
    )]
    #[case::second_bracket_after_the_closer(
        "match x:\n    case 1: y = f(\n        a\n    ) + g(\n        b\n    )\n",
        false
    )]
    fn hangs_from_its_row_reads_the_explode_layout(#[case] src: &str, #[case] hangs: bool) {
        let source = parse(src);
        let Stmt::Match(m) = &source.ast().body[0] else {
            panic!("the source opens on a match");
        };
        let case = &m.cases[0];
        assert_eq!(
            hangs_from_its_row(&source, &case.body[0], case.start()),
            hangs
        );
    }
}
