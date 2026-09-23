//! Walks match statements collecting the case arms that align.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    MatchCase, Stmt, StmtMatch,
    helpers::is_compound_statement,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::*;

pub(super) struct Visitor<'a> {
    pub(super) code_line_length: usize,
    pub(super) settling: Settling,
    pub(super) walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Emits the alignment and collapse edits for `arms` as one fix
    /// group. Where an arm's folded line at the run's column would exceed
    /// `code_line_length`, records [`Self::unfold_edits`] for the first
    /// such arm instead and emits the arms on either side as runs of their own.
    fn emit_run(&mut self, arms: &[Arm]) {
        match self.first_overflow(arms) {
            None => {
                let members: Vec<_> = arms.iter().map(|arm| arm.member).collect();
                self.walker
                    .emit_group_with_gaps(&members, arms.iter().map(|arm| arm.collapse));
            }
            Some(i) => {
                self.emit_run(&arms[..i]);
                self.walker
                    .push_group(self.unfold_edits(arms[i].member, Some(arms[i].collapse)));
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

    /// Returns the display width `body` takes through the end of its row
    /// once folded, reading a trailing comment at the width the enabled
    /// comment rules settle it to and any other tail as written.
    fn folded_width(&self, body: &Stmt, member: aligner::Member) -> usize {
        let source = self.walker.source;
        let slack = trailing_comment(source, body.end()).map_or(0, |comment| {
            let gap = aligner::line_gap_before(source, comment.start());
            self.settling.slack(source, comment, gap, member.gap)
        });
        source
            .width_between(body.start(), source.row_tail(body.end()).end())
            .saturating_add_signed(-slack)
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
    /// for a pattern spanning lines, a multi-statement, compound, or
    /// multi-line body, or a comment in the `:`-to-body gap, and
    /// `Overflow` where the arm's folded line alone exceeds `code_line_length`.
    fn qualify_case(&self, case: &MatchCase) -> CaseOutcome {
        let source = self.walker.source;
        let Some(member) = colon_targets::match_case(source, case) else {
            return CaseOutcome::Disqualify(None);
        };
        let [body_first] = case.body.as_slice() else {
            return CaseOutcome::Disqualify(Some(member));
        };
        if is_compound_statement(body_first) || source.contains_line_break(body_first) {
            return CaseOutcome::Disqualify(Some(member));
        }
        let collapse = TextRange::new(member.gap.end() + TextSize::of(':'), body_first.start());
        if source.intersects_comment(collapse) {
            return CaseOutcome::Disqualify(Some(member));
        }
        let arm = Arm {
            body_width: self.folded_width(body_first, member),
            collapse,
            member,
        };
        if self.first_overflow(std::slice::from_ref(&arm)).is_some() {
            return CaseOutcome::Overflow(arm);
        }
        CaseOutcome::Align(arm)
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

/// An arm whose single-statement body can fold onto its `case` line.
struct Arm {
    /// The display width [`Visitor::folded_width`] measures for the
    /// body's row, a trailing comment read at its settled width.
    body_width: usize,
    /// The `:`-to-body gap the fold collapses to one space.
    collapse: TextRange,
    /// The row whose `:` aligns with the run.
    member: aligner::Member,
}

/// Outcome of qualifying one `case` arm. `Align` enrolls the arm in
/// the active run. `Disqualify` ends the run and removes the padding
/// before the arm's `:` where the `:` sits on the line the pattern
/// opens on. `Overflow` ends the run and leaves the arm multi-line
/// through [`Visitor::unfold_edits`], splitting it where it sits folded.
enum CaseOutcome {
    Align(Arm),
    Disqualify(Option<aligner::Member>),
    Overflow(Arm),
}
