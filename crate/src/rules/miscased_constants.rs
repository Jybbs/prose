//! Flags a module-level single-name assignment whose inert value nothing
//! reassigns and whose name is not SCREAMING_CASE. The carve-outs (a
//! single-character name, a leading underscore, a `TypeAlias`-annotated
//! target, the `if TYPE_CHECKING:` block, and the per-project
//! `allow_pattern`) drop out ahead of the gate. The SCREAMING_CASE
//! rename is a display-only suggestion, and notebooks are skipped whole.

use heck::ToShoutySnakeCase;
use regex_lite::Regex;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprName, Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::Ranged;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        binding::{
            BindingAnalysis, annotated_name_target_expr, is_screaming_case,
            single_name_target_expr, skips_module_scan, tail_identifier,
        },
        effect::value_is_effectful,
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct MiscasedConstants {
    allow_pattern: Regex,
}

impl MiscasedConstants {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow_pattern: config.rules.miscased_constants.allow_pattern.clone(),
        }
    }
}

impl Rule for MiscasedConstants {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        if source.is_notebook() {
            return Vec::new();
        }
        let mut walker = Walker {
            allow_pattern: &self.allow_pattern,
            analysis: source.binding_analysis(),
            diagnostics: Vec::new(),
            rule: self.id(),
        };
        walker.visit_body(&source.ast().body);
        walker.diagnostics
    }
}

struct Walker<'a> {
    allow_pattern: &'a Regex,
    analysis: &'a BindingAnalysis,
    diagnostics: Vec<Diagnostic>,
    rule: RuleId,
}

impl Walker<'_> {
    /// True when `name` matches the configured allow pattern. The empty
    /// default pattern matches every input, so it reads as "exempt
    /// nothing" rather than "exempt everything".
    fn allow_matches(&self, name: &str) -> bool {
        !self.allow_pattern.as_str().is_empty() && self.allow_pattern.is_match(name)
    }

    fn emit(&mut self, target: &ExprName) {
        let name = target.id.as_str();
        self.diagnostics.push(Diagnostic::suggestion(
            self.rule,
            target.range(),
            format!("Module constant `{name}` is not SCREAMING_CASE"),
            Edit::range_replacement(name.to_shouty_snake_case(), target.range()),
        ));
    }

    fn flag_if_miscased(&mut self, target: &ExprName, value: &Expr, annotation: Option<&Expr>) {
        if self.is_miscased(target.id.as_str(), value, annotation) {
            self.emit(target);
        }
    }

    /// True when `name` is a module constant miscased against
    /// SCREAMING_CASE: a multi-character name with an inert `value`, no
    /// leading underscore, not already SCREAMING_CASE, never reassigned,
    /// and outside the `TypeAlias`-annotation and allow-pattern
    /// exemptions. A single-character name is spared, its lone-capital
    /// SCREAMING form reading as a matrix and its lowercase form usually
    /// a mathematical scalar.
    fn is_miscased(&self, name: &str, value: &Expr, annotation: Option<&Expr>) -> bool {
        name.chars().count() > 1
            && !name.starts_with('_')
            && !is_screaming_case(name)
            && !self.analysis.module_reassigned(name)
            && !annotation.is_some_and(is_type_alias)
            && !self.allow_matches(name)
            && !value_is_effectful(value)
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if skips_module_scan(stmt) {
            return;
        }
        match stmt {
            Stmt::Assign(a) => {
                if let Some(target) = single_name_target_expr(a) {
                    self.flag_if_miscased(target, a.value.as_ref(), None);
                }
            }
            Stmt::AnnAssign(a) => {
                if let Some(target) = annotated_name_target_expr(a)
                    && let Some(value) = a.value.as_deref()
                {
                    self.flag_if_miscased(target, value, Some(a.annotation.as_ref()));
                }
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// True when `annotation` names `TypeAlias`, bare or attribute-qualified
/// (`typing.TypeAlias`).
fn is_type_alias(annotation: &Expr) -> bool {
    tail_identifier(annotation) == Some("TypeAlias")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Applicability;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::testing::{notebook, parse};

    fn rule() -> MiscasedConstants {
        MiscasedConstants::from_config(&Config::default())
    }

    #[test]
    fn diagnostic_pins_severity_display_only_rename_and_range() {
        let source = parse("max_retries = 5\n");
        let diagnostics = rule().lint(&source);
        let only = diagnostics.first().expect("one diagnostic");

        assert_eq!(only.severity, Severity::Lint);
        let fix = only.fix.as_ref().expect("display-only rename");
        assert_eq!(fix.applicability(), Applicability::DisplayOnly);
        assert_eq!(fix.edits()[0].content(), Some("MAX_RETRIES"));
        assert!(only.message.contains("`max_retries`"));
        assert_eq!(&source.text()[only.range], "max_retries");
    }

    #[rstest]
    #[case("x: TypeAlias", true)]
    #[case("x: typing.TypeAlias", true)]
    #[case("x: int", false)]
    fn is_type_alias_matches_bare_and_qualified(#[case] src: &str, #[case] expected: bool) {
        let source = parse(&format!("{src} = 0\n"));
        let annotation = source.ast().body[0]
            .as_ann_assign_stmt()
            .expect("an annotated assignment")
            .annotation
            .as_ref();
        assert_eq!(is_type_alias(annotation), expected);
    }

    #[test]
    fn notebook_cells_are_skipped() {
        let source = notebook(&["max_retries = 5\n"]);
        assert!(rule().lint(&source).is_empty());
    }

    #[rstest]
    #[case("maxRetries = 5\n", "MAX_RETRIES")]
    #[case("MaxRetries = 5\n", "MAX_RETRIES")]
    fn rename_folds_camel_and_pascal_to_screaming(#[case] src: &str, #[case] expected: &str) {
        let diagnostics = rule().lint(&parse(src));
        let fix = diagnostics
            .first()
            .expect("one diagnostic")
            .fix
            .as_ref()
            .expect("a fix");
        assert_eq!(fix.edits()[0].content(), Some(expected));
    }

    #[rstest]
    fn single_character_names_are_spared(
        #[values("x = 3\n", "n = 100\n", "e = 2.718\n")] src: &str,
    ) {
        assert!(rule().lint(&parse(src)).is_empty(), "{src}");
    }
}
