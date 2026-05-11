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

    fn lint_default(text: &str) -> Vec<Diagnostic> {
        let rule = BareImportAllowlist::from_config(&Config::default());
        rule.lint(&parse(text))
    }

    #[test]
    fn aliased_import_is_flagged_on_the_top_level_segment() {
        let diagnostics = lint_default("import os as o\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("`os`"));
    }

    #[test]
    fn allowlisted_top_level_imports_emit_no_diagnostic() {
        assert!(lint_default("import numpy\nimport pandas\n").is_empty());
    }

    #[test]
    fn diagnostic_carries_lint_severity_and_no_fix() {
        let diagnostics = lint_default("import os\n");
        let only = diagnostics.first().expect("one diagnostic");
        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
    }

    #[test]
    fn dotted_path_inherits_top_level_allowlist_membership() {
        assert!(lint_default("import numpy.linalg\n").is_empty());
    }

    #[test]
    fn empty_allowlist_flags_every_bare_import() {
        let mut config = Config::default();
        config.rules.bare_import_allowlist.allow.clear();
        let rule = BareImportAllowlist::from_config(&config);

        let diagnostics = rule.lint(&parse("import numpy\nimport pandas\nimport os\n"));

        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn from_import_emits_no_diagnostic_for_a_non_allowlisted_module() {
        assert!(lint_default("from os import path\n").is_empty());
    }

    #[test]
    fn multi_alias_import_flags_each_non_allowlisted_segment() {
        let text = "import os, sys, numpy\n";
        let flagged: Vec<&str> = lint_default(text)
            .iter()
            .map(|d| &text[d.range])
            .collect();
        assert_eq!(flagged, ["os", "sys"]);
    }

    #[test]
    fn non_allowlisted_bare_import_is_flagged() {
        let diagnostics = lint_default("import os\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("`os`"));
        assert!(diagnostics[0].message.contains("from os import"));
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
