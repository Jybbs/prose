//! Flags function-local bindings that are written exactly once and
//! read exactly once, where inlining the right-hand side into the use
//! site is usually more direct. Lint-only, emits no edits.
//!
//! Conservative skips absorb the false-positive surfaces:
//!
//! - Functions whose body declares `global` or `nonlocal` are skipped
//!   entirely, since the scope analysis becomes cross-function.
//! - Comprehension targets are skipped, since their bindings live in
//!   the comprehension's own scope rather than the enclosing function.
//! - Augmented assignments are skipped, since `x += 1` is both a read
//!   and a write of `x`.
//! - Names matching the configurable `allow_pattern` glob (default
//!   `_*`) are skipped, exempting `_unused` and similar, whereas an
//!   empty pattern exempts nothing.
//! - Only `Assignment` and `Walrus` writes flag, leaving parameters,
//!   loop targets, `with`-targets, exception handlers, and nested
//!   `def`/`class` bindings out of the diagnostic surface.
//! - A single-use tuple-unpack target is exempt when a sibling reads
//!   more than once, since removing it would split the unpack into an
//!   indexed read.
//! - A walrus bound in the test of an `if`, `elif`, or `while` is
//!   exempt, since the test consumes the value and the single later
//!   read is the second use.

use ruff_python_ast::statement_visitor::StatementVisitor;

mod walk;

use walk::Visitor;

use crate::{
    config::{AllowPattern, Config},
    diagnostics::Diagnostic,
    primitives::{
        binding::{BindingAnalysis, BindingId, BindingKind, UnpackKind},
        walk::any_over_stmts,
    },
    rule::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct SingleUseVariables {
    allow_pattern: AllowPattern,
}

impl SingleUseVariables {
    pub(crate) const MESSAGE: &'static str = "Binding is assigned and used once. Consider inlining";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow_pattern: config.rules.single_use_variables.allow_pattern.clone(),
        }
    }
}

impl Rule for SingleUseVariables {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut visitor = Visitor {
            allow_pattern: &self.allow_pattern,
            analysis: source.binding_analysis(),
            diagnostics: Vec::new(),
            rule: self.id(),
            text: source.text(),
        };
        visitor.visit_body(&source.ast().body);
        visitor.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diagnostics::Severity, testing::parse};

    #[test]
    fn an_empty_allow_pattern_exempts_nothing() {
        let mut config = Config::default();
        config.rules.single_use_variables.allow_pattern = "".parse().expect("empty pattern parses");
        let source = parse("def f():\n    _unused = 1\n    return _unused\n");
        let diagnostics = SingleUseVariables::from_config(&config).lint(&source);
        assert!(
            !diagnostics.is_empty(),
            "the default `_*` would spare `_unused`, and an empty pattern spares nothing",
        );
    }

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_range_over_name() {
        let source = parse("def f():\n    x = 1\n    return x\n");
        let rule = SingleUseVariables::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert!(only.message.contains("`x`"));
        assert!(only.message.ends_with("Consider inlining `1`"));
        assert_eq!(&source.text()[only.range], "x");
    }

    #[test]
    fn message_carries_inlined_value_from_nested_block() {
        let source = parse("def f():\n    if cond:\n        y = g() + 1\n        return y\n");
        let rule = SingleUseVariables::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert!(only.message.ends_with("Consider inlining `g() + 1`"));
    }

    #[test]
    fn walrus_binding_message_omits_unnameable_value() {
        let source = parse("def f(items):\n    print(n := len(items))\n    return n\n");
        let rule = SingleUseVariables::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert!(only.message.ends_with("Consider inlining"));
        assert!(!only.message.contains("inlining `"));
    }
}
