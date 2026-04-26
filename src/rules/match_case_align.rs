//! Collapses each arm of a `match` statement to one source line of
//! the form `case PATTERN : EXPR` and aligns the `:` column across
//! arms whose body is a single collapsible statement on a single
//! source line. A disqualifying arm (multi-statement body,
//! compound-statement body, multi-line body, or a comment between
//! the `:` and the body) breaks alignment into sub-groups on either
//! side. Each sub-group runs the configured `Split` / `Drop` / `Skip`
//! policy independently. Singleton sub-groups collapse without
//! pre-colon padding. Nested matches recurse.

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_compound_statement;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::{MatchCase, Stmt, StmtMatch};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::config::Config;
use crate::pipeline::Rule;
use crate::primitives::{aligner, colon_targets};
use crate::source::Source;

pub struct MatchCaseAlign {
    settings: aligner::Settings,
}

impl MatchCaseAlign {
    pub fn from_config(config: &Config) -> Self {
        Self {
            settings: aligner::Settings::from(&config.rules.match_case_align)
                .with_singleton_subgroup_strip(),
        }
    }
}

impl Rule for MatchCaseAlign {
    fn apply(&self, source: &Source) -> Vec<Edit> {
        let mut visitor = Visitor {
            edits: Vec::new(),
            settings: self.settings,
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.edits
    }

    fn name(&self) -> &'static str {
        "match-case-align"
    }
}

struct Visitor<'a> {
    edits: Vec<Edit>,
    settings: aligner::Settings,
    source: &'a Source,
}

impl Visitor<'_> {
    /// Emits alignment and collapse edits for a sub-group and drains it.
    fn flush_subgroup(&mut self, group: &mut Vec<(aligner::Member, TextRange)>) {
        let (members, ranges): (Vec<aligner::Member>, Vec<TextRange>) = group.drain(..).unzip();
        if members.len() >= 2 {
            aligner::emit_group(self.source, &members, self.settings, &mut self.edits);
        }
        self.edits.extend(
            ranges
                .into_iter()
                .filter(|range| self.source.slice(*range) != " ")
                .map(|range| Edit::range_replacement(" ".to_owned(), range)),
        );
    }

    /// Emits collapse-and-align edits for one match. Sub-groups
    /// form at disqualifying-arm boundaries. Each multi-member
    /// sub-group runs through `aligner::emit_group`, and every
    /// qualifying arm gets a collapse edit. Singleton sub-groups
    /// skip alignment.
    fn process_match(&mut self, m: &StmtMatch) {
        let mut current: Vec<(aligner::Member, TextRange)> = Vec::new();
        for case in &m.cases {
            match self.qualify_case(case) {
                Some(target) => current.push(target),
                None => self.flush_subgroup(&mut current),
            }
        }
        self.flush_subgroup(&mut current);
    }

    /// Returns the alignment member and the post-colon collapse
    /// range for a qualifying arm, or `None` when the arm
    /// disqualifies. The collapse range covers the whitespace span
    /// between the `:` and the body's first statement.
    fn qualify_case(&self, case: &MatchCase) -> Option<(aligner::Member, TextRange)> {
        let [body_first] = case.body.as_slice() else {
            return None;
        };
        if is_compound_statement(body_first)
            || self.source.contains_line_break(body_first.range())
        {
            return None;
        }
        let member = colon_targets::match_case(self.source, case)?;
        let collapse_range =
            TextRange::new(member.gap.end() + TextSize::from(1u32), body_first.start());
        (!self.source.intersects_comment(collapse_range)).then_some((member, collapse_range))
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
    use std::str::FromStr;

    use super::*;

    /// Parses `src` as a module and tests whether its first top-level
    /// statement would qualify a `match` arm.
    fn collapsible(src: &str) -> bool {
        let s = Source::from_str(src).expect("test source parses");
        !is_compound_statement(&s.ast().body[0])
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
