//! Flags every `import X` whose top-level segment is not in the
//! configured allowlist. Lint-only, emits no edits.

use std::collections::HashSet;

use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::top_level_module,
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct BareImports {
    allow: HashSet<String>,
}

impl BareImports {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: config.rules.bare_imports.allow.iter().cloned().collect(),
        }
    }
}

impl Rule for BareImports {
    fn id(&self) -> RuleId {
        Self::SLUG
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

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Import(import) = stmt {
            for alias in &import.names {
                let name = alias.name.as_str();
                if !self.allow.contains(top_level_module(name)) {
                    self.diagnostics.push(Diagnostic::lint(
                        self.rule,
                        alias.name.range,
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::test_support::parse;

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_message_format() {
        let rule = BareImports::from_config(&Config::default());
        let diagnostics = rule.lint(&parse("import os\n"));
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.contains("`os`"));
        assert!(only.message.contains("from os import"));
    }
}
