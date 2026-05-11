//! Flags every `import X` whose top-level segment is not in the
//! configured allowlist. Lint-only, emits no edits.

use std::collections::HashSet;

use ruff_diagnostics::Edit;
use ruff_python_ast::statement_visitor::{walk_stmt, StatementVisitor};
use ruff_python_ast::Stmt;

use crate::config::Config;
use crate::diagnostics::Diagnostic;
use crate::rule::{Rule, RuleId};
use crate::source::Source;

pub(crate) struct BareImportAllowlist {
    allow: HashSet<String>,
}

impl BareImportAllowlist {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: config
                .rules
                .bare_import_allowlist
                .allow
                .iter()
                .cloned()
                .collect(),
        }
    }
}

impl Rule for BareImportAllowlist {
    fn apply(&self, _source: &Source) -> Vec<Edit> {
        Vec::new()
    }

    fn id(&self) -> RuleId {
        RuleId::from(ruff_macros::kebab_case!(BareImportAllowlist))
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut visitor = Visitor {
            allow: &self.allow,
            diagnostics: Vec::new(),
            rule: self.id(),
        };
        visitor.visit_body(&source.ast().body);
        visitor.diagnostics
    }
}

struct Visitor<'a> {
    allow: &'a HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    rule: RuleId,
}

impl<'a> StatementVisitor<'a> for Visitor<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Import(import) = stmt {
            for alias in &import.names {
                let name = alias.name.as_str();
                if !self.allow.contains(top_level_segment(name)) {
                    self.diagnostics.push(Diagnostic::lint(
                        self.rule,
                        alias.range,
                        format!(
                            "bare import `{name}` outside allowlist. Rewrite as `from {name} import ...` listing only the symbols this module uses",
                        ),
                    ));
                }
            }
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the substring of `name` preceding the first `.`, or all of
/// `name` when no `.` is present. `numpy.linalg` resolves to `numpy`.
fn top_level_segment(name: &str) -> &str {
    name.split_once('.').map_or(name, |(top, _)| top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;
    use crate::test_support::parse;

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_message_format() {
        let rule = BareImportAllowlist::from_config(&Config::default());
        let diagnostics = rule.lint(&parse("import os\n"));
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.contains("`os`"));
        assert!(only.message.contains("from os import"));
    }

    #[test]
    fn top_level_segment_returns_input_for_a_non_dotted_name() {
        assert_eq!(top_level_segment("os"), "os");
    }

    #[test]
    fn top_level_segment_returns_prefix_for_a_dotted_name() {
        assert_eq!(top_level_segment("numpy.linalg"), "numpy");
    }
}
