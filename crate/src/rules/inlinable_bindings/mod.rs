//! Flags a function-local binding written once and read once whose
//! value inlines at the read for free. Lint-only, emits no edits.
//!
//! A candidate is declined where no replacement text resolves, where
//! the value spans rows, where the read sits inside a region `guards`
//! names that the write sits outside of, and where the swap carries
//! the read's row past `code_line_length`. Names matching
//! `allow_pattern`, names a `del` statement targets, a function
//! declaring `global` or `nonlocal`, and every write kind but
//! `Assignment` and `Walrus` stay outside the surface.

use ruff_python_ast::statement_visitor::StatementVisitor;

use crate::{
    config::{AllowPattern, Config},
    diagnostics::Diagnostic,
    primitives::binding::{BindingAnalysis, BindingId, BindingKind, UnpackKind},
    rule::{Rule, RuleId},
    source::Source,
};

mod guards;
mod walk;

use self::walk::Visitor;

#[derive(Debug)]
pub(crate) struct InlinableBindings {
    allow_pattern: AllowPattern,
    code_line_length: usize,
}

impl InlinableBindings {
    pub(crate) const MESSAGE: &'static str = "Flag a binding assigned and read once whose value inlines at the read without recomputing it, moving it under a guard, or crossing the line budget";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow_pattern: config.rules.inlinable_bindings.allow_pattern.clone(),
            code_line_length: config.code_width(),
        }
    }
}

impl Rule for InlinableBindings {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut visitor = Visitor {
            allow_pattern: &self.allow_pattern,
            analysis: source.binding_analysis(),
            code_line_length: self.code_line_length,
            diagnostics: Vec::new(),
            rule: self.id(),
            source,
        };
        visitor.visit_body(&source.ast().body);
        visitor.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::{diagnostics::Severity, testing::parse};

    fn rule() -> InlinableBindings {
        InlinableBindings::from_config(&Config::default())
    }

    #[rstest]
    #[case::lands_on_the_cap("ab(cdefg)", true)]
    #[case::lands_one_past_the_cap("ab(cdefgh)", false)]
    fn a_swap_landing_on_the_cap_still_reports(#[case] value: &str, #[case] reports: bool) {
        let config = Config {
            code_line_length: NonZeroUsize::new(20),
            ..Config::default()
        };
        let source = parse(&format!("def f():\n    x = {value}\n    return x\n"));
        let diagnostics = InlinableBindings::from_config(&config).lint(&source);

        assert_eq!(!diagnostics.is_empty(), reports);
    }

    #[test]
    fn an_empty_allow_pattern_exempts_nothing() {
        let mut config = Config::default();
        config.rules.inlinable_bindings.allow_pattern = "".parse().expect("empty pattern parses");
        let source = parse("def f():\n    _unused = 1\n    return _unused\n");
        let diagnostics = InlinableBindings::from_config(&config).lint(&source);
        assert!(
            !diagnostics.is_empty(),
            "the default `_*` would spare `_unused`, and an empty pattern spares nothing",
        );
    }

    #[test]
    fn diagnostic_shape_pins_severity_no_fix_and_range_over_name() {
        let source = parse("def f():\n    x = 1\n    return x\n");
        let diagnostics = rule().lint(&source);
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
        let diagnostics = rule().lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert!(only.message.ends_with("Consider inlining `g() + 1`"));
    }
}
