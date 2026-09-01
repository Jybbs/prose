//! Rewrites a legacy `typing` annotation to the spelling the target
//! runtime supports, converting a `typing` generic to the builtin
//! PEP 585 gave it under `rewrite-generics` and an `Optional` or `Union`
//! subscript to the PEP 604 pipe union under `rewrite-unions`, then
//! dropping every `typing` import the rewrites leave unread.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, PythonVersion};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    primitives::{
        edit::{narrowed_replacement, singleton_groups},
        walk::{Descent, ParentedProbe, walk_parented_exprs},
    },
    rule::{Rule, RuleId},
    rules::reflow_imports::Folds,
    source::Source,
};

mod render;
mod typing_imports;

use render::Renderer;
use typing_imports::TypingImports;

#[derive(Debug)]
pub(crate) struct ModernizeAnnotations {
    folds: Folds,
    generics: bool,
    unions: bool,
}

impl ModernizeAnnotations {
    pub(crate) const MESSAGE: &'static str =
        "modernize a legacy `typing` annotation to its builtin or PEP 604 form";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let facets = &config.rules.modernize_annotations;
        let targets = |version: PythonVersion| {
            config
                .target_version
                .is_some_and(|target| target >= version)
        };
        Self {
            folds: Folds::from_config(config),
            generics: facets.rewrite_generics && targets(PythonVersion::PY39),
            unions: facets.rewrite_unions && targets(PythonVersion::PY310),
        }
    }
}

impl Rule for ModernizeAnnotations {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        if !self.generics && !self.unions {
            return Vec::new();
        }
        let Some(imports) = TypingImports::collect(&source.ast().body) else {
            return Vec::new();
        };
        let mut walker = Walker {
            consumed: FxHashMap::default(),
            edits: Vec::new(),
            renderer: Renderer::new(source, imports.aliases(), self.generics, self.unions),
            source,
        };
        walk_parented_exprs(source.ast(), &mut walker);
        let mut groups = singleton_groups(walker.edits);
        groups.extend(imports.prune(source, &walker.consumed, &self.folds));
        groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Rewrites the outermost expression whose rendered form differs,
/// counting the bound name each rewritten head read.
struct Walker<'a> {
    consumed: FxHashMap<&'a str, usize>,
    edits: Vec<Edit>,
    renderer: Renderer<'a>,
    source: &'a Source,
}

impl<'a> Walker<'a> {
    /// Rewrites `expr` whole against the enclosing `parent`, returning
    /// `true` once the walk may leave the members the edit already
    /// covers unvisited. A suppressed edit is dropped without counting
    /// its heads, leaving the import it reads through in place.
    fn rewrite(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>) -> bool {
        let Some(rewrite) = self.renderer.rewrite(expr) else {
            return false;
        };
        let span = self.source.paren_aware_range(expr.into(), parent);
        let Some(edit) = narrowed_replacement(self.source, span, rewrite.text) else {
            return false;
        };
        if !self
            .source
            .suppression_map()
            .suppresses(&edit, ModernizeAnnotations::SLUG)
        {
            for bound in rewrite.consumed {
                *self.consumed.entry(bound).or_default() += 1;
            }
            self.edits.push(edit);
        }
        true
    }
}

impl<'a> ParentedProbe<'a> for Walker<'a> {
    fn probe(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>, _: &[AnyNodeRef<'a>]) -> Descent {
        if self.rewrite(expr, parent) {
            Descent::Over
        } else {
            Descent::Into
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::parse;

    fn rule(version: PythonVersion) -> ModernizeAnnotations {
        ModernizeAnnotations::from_config(&Config {
            target_version: Some(version),
            ..Config::default()
        })
    }

    #[test]
    fn a_dotted_bare_import_holds_while_its_head_rewrites() {
        let source = parse("import typing.io\n\nx: typing.List[int] = []\n");
        let groups = rule(PythonVersion::PY39).apply(&source);
        assert_eq!(groups.len(), 1, "the head rewrites and the import holds");
    }

    #[test]
    fn a_module_without_a_typing_import_emits_no_edits() {
        let source = parse("x: int | None = None\n");
        assert!(rule(PythonVersion::PY314).apply(&source).is_empty());
    }

    #[test]
    fn a_relative_typing_import_is_not_the_stdlib_module() {
        let source = parse("from .typing import Optional\n\nx: Optional[int] = None\n");
        assert!(rule(PythonVersion::PY310).apply(&source).is_empty());
    }

    #[test]
    fn a_starred_member_holds_the_union() {
        let source = parse("from typing import Union\n\nx: Union[*Ts] = None\n");
        assert!(rule(PythonVersion::PY310).apply(&source).is_empty());
    }

    #[test]
    fn an_alias_rebinding_a_builtin_name_emits_no_edits() {
        let source = parse("from typing import List as list\n\nx: list[int] = []\n");
        assert!(rule(PythonVersion::PY310).apply(&source).is_empty());
    }

    #[test]
    fn an_empty_union_slice_holds() {
        let source = parse("from typing import Optional\n\nx: Optional[()] = None\n");
        assert!(rule(PythonVersion::PY310).apply(&source).is_empty());
    }

    #[test]
    fn both_facets_disabled_emit_no_edits() {
        let mut config = Config {
            target_version: Some(PythonVersion::PY314),
            ..Config::default()
        };
        config.rules.modernize_annotations.rewrite_generics = false;
        config.rules.modernize_annotations.rewrite_unions = false;
        let source = parse("from typing import List, Optional\n\nx: Optional[List[int]] = None\n");
        assert!(
            ModernizeAnnotations::from_config(&config)
                .apply(&source)
                .is_empty()
        );
    }
}
