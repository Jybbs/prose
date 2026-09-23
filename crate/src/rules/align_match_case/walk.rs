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
    pub(super) walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Emits the alignment and collapse edits for `arms` as one fix
    /// group. Keeps an arm multi-line instead where its fold at the
    /// run's column would exceed `code_line_length`, emitting the arms
    /// above and below it as separate runs.
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
    /// would exceed `code_line_length` once its `:` pads to the column
    /// the run gives it.
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

    /// Classifies one arm into the matching `CaseOutcome` variant.
    /// Disqualifies on a pattern spanning lines, on multi-statement,
    /// compound, or multi-line bodies, and on a comment in the
    /// `:`-to-body gap, and reports an arm whose folded line would
    /// exceed `code_line_length` even outside a run as an overflow.
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
            body_width: source.width_between(body_first.start(), body_first.end())
                + trailing_comment(source, body_first.end())
                    .map_or(0, |comment| trailing_width(source, comment)),
            collapse,
            member,
        };
        if self.first_overflow(std::slice::from_ref(&arm)).is_some() {
            return CaseOutcome::Overflow(arm);
        }
        CaseOutcome::Align(arm)
    }

    /// Returns the edits holding an arm out of every run, drawing its
    /// `:` flush against the pattern and pushing its body onto the next
    /// line where a `collapse` gap is given on the `case` line.
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

/// One arm whose single-statement body can fold onto its `case` line,
/// carrying the row its `:` aligns on, the `:`-to-body gap its fold
/// collapses to one space, and the display width of the body the fold
/// joins onto that line, counting any trailing comment on the body's
/// line at the two-space gap the comment rules settle it to.
struct Arm {
    body_width: usize,
    collapse: TextRange,
    member: aligner::Member,
}

/// Outcome of qualifying one `case` arm. `Align` enrolls the arm in
/// the active run. `Disqualify` breaks the run and draws the arm's `:`
/// flush where it shares the pattern's line. `Overflow` breaks the run
/// and keeps the arm multi-line through [`Visitor::unfold_edits`].
enum CaseOutcome {
    Align(Arm),
    Disqualify(Option<aligner::Member>),
    Overflow(Arm),
}
