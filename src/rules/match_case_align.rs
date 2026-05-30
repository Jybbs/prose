//! Collapses each `match` arm to a one-line `case PATTERN : EXPR`
//! and aligns the `:` column across arms whose body is a single
//! collapsible statement on one source line. A disqualifying arm
//! (multi-statement body, compound-statement body, multi-line body,
//! or a comment between the `:` and the body) breaks alignment into
//! sub-groups on either side. An arm whose collapsed form would
//! exceed `Config::code_line_length` also disqualifies, and any
//! such arm that sits on one source line splits so the body lands
//! on the next line. Nested matches recurse.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    MatchCase, Stmt, StmtMatch,
    helpers::is_compound_statement,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{INDENT_STEP, aligner, colon_targets},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct MatchCaseAlign {
    code_line_length: usize,
    settings: aligner::Settings,
}

impl MatchCaseAlign {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            settings: aligner::Settings::from(&config.rules.match_case_align)
                .with_singleton_subgroup_strip(),
        }
    }
}

impl Rule for MatchCaseAlign {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut visitor = Visitor {
            code_line_length: self.code_line_length,
            walker: aligner::AlignWalker::new(source, self.settings, Self::SLUG),
        };
        visitor.visit_body(&source.ast().body);
        visitor.walker.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Outcome of qualifying one `case` arm. `Align` enrolls the arm in
/// the active alignment sub-group. `Disqualify` breaks the sub-group
/// without emitting an edit. `Split` breaks the sub-group and emits
/// an edit pushing the body onto the next source line.
enum CaseOutcome {
    Align(aligner::Member, TextRange),
    Disqualify,
    Split(Edit),
}

struct Visitor<'a> {
    code_line_length: usize,
    walker: aligner::AlignWalker<'a>,
}

impl Visitor<'_> {
    /// Emits a sub-group's alignment and collapse edits as one fix
    /// group and drains it.
    fn flush_subgroup(&mut self, group: &mut Vec<(aligner::Member, TextRange)>) {
        let (members, ranges): (Vec<aligner::Member>, Vec<TextRange>) = group.drain(..).unzip();
        let mut edits = self.walker.group_edits(&members);
        edits.extend(
            ranges
                .into_iter()
                .filter_map(|r| aligner::space_padding_edit(self.walker.source, r, 1)),
        );
        self.walker.push_group(edits);
    }

    /// Returns `Disqualify` when the `:`-to-body gap already carries
    /// a line break, otherwise `Split` carrying an edit that pushes
    /// the body onto the next source line.
    fn over_budget_outcome(&self, case: &MatchCase, collapse_range: TextRange) -> CaseOutcome {
        if self.walker.source.contains_line_break(collapse_range) {
            return CaseOutcome::Disqualify;
        }
        let body_indent = self.walker.source.line_indent_width(case.start()) + INDENT_STEP;
        let replacement = format!(
            "{}{}",
            self.walker.source.newline_str(),
            " ".repeat(body_indent),
        );
        CaseOutcome::Split(Edit::range_replacement(replacement, collapse_range))
    }

    /// Returns `true` when the canonical
    /// `[indent]case PATTERN[ if GUARD] : BODY` form for `case` would
    /// exceed `code_line_length`.
    fn overflows_budget(&self, case: &MatchCase, body_first: &Stmt) -> bool {
        let pre_colon_end = case
            .guard
            .as_deref()
            .map_or(case.pattern.end(), Ranged::end);
        let lhs_width = self
            .walker
            .source
            .slice(TextRange::new(case.start(), pre_colon_end))
            .width();
        let body_width = self.walker.source.slice(body_first.range()).width();
        self.walker.source.column_of(case.start()) + lhs_width + 3 + body_width
            > self.code_line_length
    }

    /// Emits collapse-and-align edits for one match by walking each
    /// case through `qualify_case` and dispatching on its outcome.
    fn process_match(&mut self, m: &StmtMatch) {
        let mut current: Vec<(aligner::Member, TextRange)> = Vec::new();
        for case in &m.cases {
            if self.walker.is_held(case.start()) {
                continue;
            }
            match self.qualify_case(case) {
                CaseOutcome::Align(member, range) => current.push((member, range)),
                CaseOutcome::Disqualify => self.flush_subgroup(&mut current),
                CaseOutcome::Split(edit) => {
                    self.flush_subgroup(&mut current);
                    self.walker.push_group(vec![edit]);
                }
            }
        }
        self.flush_subgroup(&mut current);
    }

    /// Classifies one arm into the matching `CaseOutcome` variant.
    /// Disqualifies on multi-statement, compound, or multi-line
    /// bodies and on a comment in the `:`-to-body gap, deferring to
    /// `over_budget_outcome` when the arm would overflow the
    /// line-length budget collapsed.
    fn qualify_case(&self, case: &MatchCase) -> CaseOutcome {
        let [body_first] = case.body.as_slice() else {
            return CaseOutcome::Disqualify;
        };
        if is_compound_statement(body_first) || self.walker.source.contains_line_break(body_first) {
            return CaseOutcome::Disqualify;
        }
        let Some(member) = colon_targets::match_case(self.walker.source, case) else {
            return CaseOutcome::Disqualify;
        };
        let collapse_range =
            TextRange::new(member.gap.end() + TextSize::from(1u32), body_first.start());
        if self.walker.source.intersects_comment(collapse_range) {
            return CaseOutcome::Disqualify;
        }
        if self.overflows_budget(case, body_first) {
            return self.over_budget_outcome(case, collapse_range);
        }
        CaseOutcome::Align(member, collapse_range)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse;

    /// Parses `src` as a module and tests whether its first top-level
    /// statement would qualify a `match` arm.
    fn collapsible(src: &str) -> bool {
        !is_compound_statement(&parse(src).ast().body[0])
    }

    #[test]
    fn collapsible_admits_assignments_and_simple_statements() {
        assert!(collapsible("x = 1\n"));
        assert!(collapsible("x: int = 1\n"));
        assert!(collapsible("x += 1\n"));
        assert!(collapsible("x\n"));
        assert!(collapsible("return x\n"));
        assert!(collapsible("raise ValueError\n"));
        assert!(collapsible("pass\n"));
        assert!(collapsible("break\n"));
        assert!(collapsible("continue\n"));
    }

    #[test]
    fn collapsible_admits_uncommon_one_liners() {
        assert!(collapsible("import x\n"));
        assert!(collapsible("from m import x\n"));
        assert!(collapsible("del x\n"));
        assert!(collapsible("global x\n"));
        assert!(collapsible("assert x\n"));
        assert!(collapsible("type X = int\n"));
    }

    #[test]
    fn collapsible_rejects_compound_statements() {
        assert!(!collapsible("if x:\n    y\n"));
        assert!(!collapsible("for i in xs:\n    y\n"));
        assert!(!collapsible("with x():\n    y\n"));
        assert!(!collapsible("match x:\n    case _:\n        y\n"));
        assert!(!collapsible("class C:\n    pass\n"));
        assert!(!collapsible("def f():\n    pass\n"));
    }
}
