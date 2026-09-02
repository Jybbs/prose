//! Flags a module-level single-name assignment whose value is inert,
//! whose name nothing reassigns, and whose name is not `SCREAMING_CASE`,
//! carving out a single-character name, a leading underscore, a
//! `TypeAlias` annotation, an alias value, a lambda, the
//! `if TYPE_CHECKING:` block, and the per-project `allow_pattern`. The
//! rename is display-only, and notebooks are skipped whole.

use heck::ToShoutySnakeCase;
use ruff_diagnostics::Edit;
use ruff_python_ast::ExprName;
use ruff_text_size::Ranged;

use crate::{
    config::{AllowPattern, Config},
    diagnostics::Diagnostic,
    primitives::{
        alias::{AliasContext, is_type_alias},
        binding::{ModuleAssignment, is_explicit_type_alias, is_screaming_case},
        effect::value_is_effectful,
    },
    rules::{Rule, RuleId},
    source::Source,
};

#[derive(Debug)]
pub(crate) struct MiscasedConstants {
    allow_pattern: AllowPattern,
}

impl MiscasedConstants {
    pub(crate) const MESSAGE: &'static str =
        "Module constant is not SCREAMING_CASE. Rename it to the SCREAMING_CASE form";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            allow_pattern: config.rules.miscased_constants.allow_pattern.clone(),
        }
    }

    /// True when `site` binds a module constant miscased against
    /// `SCREAMING_CASE`, no carve-out from the module doc sparing it. The
    /// value must be present, inert, and neither a lambda nor a type.
    fn is_miscased(&self, site: &ModuleAssignment<'_>, ctx: &AliasContext<'_>) -> bool {
        let name = site.target.id.as_str();
        name.chars().count() > 1
            && !name.starts_with('_')
            && !is_screaming_case(name)
            && !ctx.analysis().module_reassigned(name)
            && !is_explicit_type_alias(site.stmt)
            && !self.allow_pattern.matches(name)
            && !is_type_alias(site, ctx)
            && site
                .value
                .is_some_and(|value| !value.is_lambda_expr() && !value_is_effectful(value))
    }

    fn rename(&self, target: &ExprName) -> Diagnostic {
        let name = target.id.as_str();
        Diagnostic::suggestion(
            self.id(),
            target.range(),
            format!("Module constant `{name}` is not SCREAMING_CASE"),
            Edit::range_replacement(name.to_shouty_snake_case(), target.range()),
        )
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
        let ctx = AliasContext::new(&source.ast().body, source.binding_analysis());
        ctx.sites()
            .iter()
            .filter(|site| self.is_miscased(site, &ctx))
            .map(|site| self.rename(site.target))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Applicability;

    use super::*;
    use crate::{
        diagnostics::Severity,
        testing::{notebook, parse},
    };

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
