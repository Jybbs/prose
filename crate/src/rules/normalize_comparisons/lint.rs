//! The lint a test comparing against a boolean literal draws, reported
//! rather than rewritten.

use ruff_python_ast::CmpOp;
use ruff_text_size::Ranged;

use super::{NormalizeComparisons, plan::Test};
use crate::diagnostics::Diagnostic;

/// The lint a test comparing an operand against `True` or `False`
/// draws, or `None` for every other test.
pub(super) fn boolean_lint(test: Test<'_>) -> Option<Diagnostic> {
    if !matches!(test.op, CmpOp::Eq | CmpOp::NotEq) {
        return None;
    }
    let literal = test
        .left
        .as_boolean_literal_expr()
        .or_else(|| test.right.as_boolean_literal_expr())?;
    let value = if literal.value { "True" } else { "False" };
    Some(Diagnostic::lint(
        NormalizeComparisons::SLUG,
        literal.range(),
        format!(
            "Comparison against the `{value}` literal. Test the operand directly, or compare with `is` to check identity"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, diagnostics::Severity, rules::Rule, testing::parse};

    #[test]
    fn lint_pins_the_literal_range_severity_and_message() {
        let source = parse("flag == True\n");
        let diagnostics = NormalizeComparisons::from_config(&Config::default()).lint(&source);
        let only = diagnostics.first().expect("one boolean lint");

        assert_eq!(only.severity, Severity::Lint);
        assert!(only.fix.is_none());
        assert_eq!(source.slice(only.range), "True");
        assert!(only.message.contains("`True`"));
    }
}
