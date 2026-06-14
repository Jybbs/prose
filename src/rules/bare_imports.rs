//! Flags a bare `import X` the author did not alias whose namespace is
//! reached only through a handful of attributes (at most `max-attributes`,
//! default 4) and never used as the bare object. An aliased bare import
//! passes while `allow-aliased` holds, and a top-level segment on the
//! `allow` list keeps its bare form. Lint-only, emits no edits.

use std::collections::HashSet;

use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::binding::{BindingAnalysis, top_level_module},
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct BareImports {
    allow: HashSet<String>,
    allow_aliased: bool,
    max_attributes: usize,
}

impl BareImports {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow: config.rules.bare_imports.allow.iter().cloned().collect(),
            allow_aliased: config.rules.bare_imports.allow_aliased,
            max_attributes: config.rules.bare_imports.max_attributes,
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
            allow_aliased: self.allow_aliased,
            analysis: source.binding_analysis(),
            diagnostics: Vec::new(),
            max_attributes: self.max_attributes,
            rule: self.id(),
        };
        visitor.visit_body(&source.ast().body);
        visitor.diagnostics
    }
}

struct Visitor<'a> {
    allow: &'a HashSet<String>,
    allow_aliased: bool,
    analysis: &'a BindingAnalysis,
    diagnostics: Vec<Diagnostic>,
    max_attributes: usize,
    rule: RuleId,
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Import(import) = stmt {
            for alias in &import.names {
                let asname = alias.asname.as_ref();
                if asname.is_some() && self.allow_aliased {
                    continue;
                }
                let name = alias.name.as_str();
                let top = top_level_module(name);
                if self.allow.contains(top) {
                    continue;
                }
                let bound = asname.map_or(top, |id| id.as_str());
                if self.analysis.module_used_bare(bound) {
                    continue;
                }
                let attributes = self.analysis.module_attribute_count(bound);
                if (1..=self.max_attributes).contains(&attributes) {
                    self.diagnostics.push(Diagnostic::lint(
                        self.rule,
                        alias.name.range,
                        format!(
                            "Bare import `{name}` reaches {attributes} of its attributes. Rewrite as `from {name} import ...` naming only the symbols this module uses",
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
    use super::*;
    use crate::diagnostics::Severity;
    use crate::testing::parse;

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_message_format() {
        let rule = BareImports::from_config(&Config::default());
        let diagnostics = rule.lint(&parse("import os\nos.getcwd()\n"));
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.contains("`os`"));
        assert!(only.message.contains("from os import"));
    }
}
