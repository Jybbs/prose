//! The gate deciding whether `from __future__ import annotations` is
//! inert. Under a target version deferring annotation evaluation per PEP
//! 749, an annotation evaluates once something reads it after the module
//! has run, so the gate clears when every name every annotation reads is
//! bound unconditionally somewhere in the module or names a builtin.
//! Otherwise it clears when no module-scope statement declares an
//! annotation and every name every annotation reads is bound ahead of it.
//! A module carrying no annotation at all satisfies either with nothing
//! to check.

use ruff_python_ast::{Expr, PythonVersion, Stmt, helpers::any_over_expr};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextSize};

use super::PruneInertImports;
use crate::{
    primitives::{
        binding::{BindingAnalysis, BindingKind},
        scope::sub_bodies,
        slots::{slot_holding, slot_positions},
        walk::for_each_annotation,
    },
    rules::band_constants::BandConstants,
    source::Source,
};

/// What settles whether a name is bound by the time the annotation reading
/// it evaluates: the binding table and the moment of evaluation.
struct Resolution<'a> {
    analysis: &'a BindingAnalysis,
    body: &'a [Stmt],
    timing: Timing,
}

impl<'a> Resolution<'a> {
    /// Resolves annotations evaluated once the module has run, under a
    /// target naming the builtins available to them.
    fn deferred(source: &'a Source, target: PythonVersion) -> Self {
        Self {
            analysis: source.binding_analysis(),
            body: &source.ast().body,
            timing: Timing::Deferred {
                minor: target.minor,
                notebook: source.is_notebook(),
            },
        }
    }

    /// Resolves annotations evaluated where they stand, each statement
    /// seated where `bands` forecasts it once the directive is gone.
    fn eager(source: &'a Source, bands: Option<&BandConstants>, sorts_definitions: bool) -> Self {
        let body = &source.ast().body;
        Self {
            analysis: source.binding_analysis(),
            body,
            timing: Timing::Eager {
                seats: bands
                    .and_then(|rule| rule.forecast(source, body, source.module_range(), false))
                    .map_or_else(
                        || (0..body.len()).collect(),
                        |bands| slot_positions(&bands.order),
                    ),
                sorts_definitions,
            },
        }
    }

    /// True when `name`, read at `offset` in the statement at `reader`, is
    /// bound when the annotation evaluates. A `del` of the name leaves it
    /// unresolved, since the annotation evaluates against the namespace the
    /// directive's removal exposes it to.
    fn binds(&self, name: &str, reader: usize, offset: TextSize) -> bool {
        if self.analysis.is_deleted(name) {
            return false;
        }
        let write = self.analysis.first_unconditional_write(name);
        match &self.timing {
            Timing::Deferred { minor, notebook } => {
                write.is_some() || is_python_builtin(name, *minor, *notebook)
            }
            Timing::Eager {
                seats,
                sorts_definitions,
            } => {
                !(*sorts_definitions && binds_a_definition(self.analysis, name))
                    && write.is_some_and(|write| {
                        write < offset || seats[slot_of(self.body, write)] < seats[reader]
                    })
            }
        }
    }

    /// True when every annotation in the body loads only names bound when
    /// it evaluates.
    fn holds_every_annotation(&self) -> bool {
        let mut resolved = true;
        for_each_annotation(self.body, |annotation| {
            resolved &= self.resolves(annotation);
        });
        resolved
    }

    /// True when every name `annotation` loads is bound when it evaluates.
    fn resolves(&self, annotation: &Expr) -> bool {
        let reader = slot_of(self.body, annotation.start());
        !any_over_expr(annotation, &|expr: &Expr| {
            expr.as_name_expr().is_some_and(|name| {
                name.ctx.is_load() && !self.binds(name.id.as_str(), reader, name.range.start())
            })
        })
    }
}

/// When an annotation evaluates, and so which bindings it reads.
enum Timing {
    /// Once something reads the annotations after the module has run, so
    /// a name resolves wherever an unconditional write binds it or a
    /// builtin of the target's `minor` version names it.
    Deferred { minor: u8, notebook: bool },
    /// Where the annotation stands, so a name resolves where an
    /// unconditional write precedes the read as written or as seated. A
    /// binding the band seats ahead of its reader counts as bound whatever
    /// its offset, whereas a name a module-level definition binds reads as
    /// unbound while `alphabetize-siblings` sorts definitions.
    Eager {
        seats: Vec<usize>,
        sorts_definitions: bool,
    },
}

/// True when removing the `annotations` directive leaves every
/// annotation in `source` evaluating without raising, each name resolved
/// once the module has run under a target deferring evaluation and where
/// the annotation stands under any other target.
pub(super) fn annotations_are_inert(rule: &PruneInertImports, source: &Source) -> bool {
    match rule
        .target_version
        .filter(|target| target.defers_annotations())
    {
        Some(target) => Resolution::deferred(source, target).holds_every_annotation(),
        None => {
            !annotates_module_scope(&source.ast().body)
                && Resolution::eager(source, rule.folds.bands(), rule.sorts_definitions)
                    .holds_every_annotation()
        }
    }
}

/// True where a statement running at module scope declares an
/// annotation, walking each compound arm and stopping at a definition,
/// whose annotations land on its own object.
fn annotates_module_scope(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::AnnAssign(_) => true,
        Stmt::ClassDef(_) | Stmt::FunctionDef(_) => false,
        _ => sub_bodies(stmt)
            .iter()
            .any(|(nested, _)| annotates_module_scope(nested)),
    })
}

/// True when a module-level `def` or `class` writes `name`.
fn binds_a_definition(analysis: &BindingAnalysis, name: &str) -> bool {
    analysis
        .module_binding_kinds(name)
        .iter()
        .any(|kind| matches!(kind, BindingKind::ClassDef | BindingKind::FunctionDef))
}

/// The slot of the `body` statement holding `offset`, which sits inside
/// one of them.
fn slot_of(body: &[Stmt], offset: TextSize) -> usize {
    slot_holding(body, offset)
        .expect("an annotation or a module-scope write sits inside a statement")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{config::Config, testing::parse};

    const CLASS_ABOVE_ITS_READER: &str =
        "class Node:\n    pass\n\n\ndef visit(node: Node) -> Node:\n    return node\n";

    /// A function whose annotations name `Alias`, bound by an inert
    /// assignment below it that `band-constants` hoists into the
    /// leading band.
    const CONSTANT_BELOW_ITS_READER: &str =
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = int\n";

    #[rstest]
    #[case::no_annotation_anywhere("value = 1\n", None, false, false, true)]
    #[case::builtin_annotation_unresolved(
        "def f(x: int) -> int:\n    return x\n",
        None,
        false,
        false,
        false
    )]
    #[case::py313_keeps_the_directive(
        "def f(x: int) -> int:\n    return x\n",
        Some(PythonVersion::PY313),
        false,
        false,
        false
    )]
    #[case::py314_defers_evaluation(
        "def f(x: int) -> int:\n    return x\n",
        Some(PythonVersion::PY314),
        false,
        false,
        true
    )]
    #[case::module_scope_name_resolves(CLASS_ABOVE_ITS_READER, None, false, false, true)]
    #[case::sorted_definition_reads_unresolved(CLASS_ABOVE_ITS_READER, None, true, false, false)]
    #[case::import_resolves_under_the_sort(
        "from tree import Node\n\n\ndef visit(node: Node) -> Node:\n    return node\n",
        None,
        true,
        false,
        true
    )]
    #[case::py314_defers_evaluation_under_the_sort(
        CLASS_ABOVE_ITS_READER,
        Some(PythonVersion::PY314),
        true,
        false,
        true
    )]
    #[case::py314_resolves_a_name_bound_below_its_reader(
        CONSTANT_BELOW_ITS_READER,
        Some(PythonVersion::PY314),
        false,
        false,
        true
    )]
    #[case::py314_resolves_a_module_scope_annotation(
        "from typing import Final\n\nLIMIT: Final = 3\n",
        Some(PythonVersion::PY314),
        false,
        false,
        true
    )]
    #[case::py314_holds_a_type_checking_import(
        "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from typing import IO\n\ntrace: IO[str] | None = None\n",
        Some(PythonVersion::PY314),
        false,
        false,
        false
    )]
    #[case::py314_holds_a_name_never_bound(
        "def f(x: Missing) -> None:\n    return None\n",
        Some(PythonVersion::PY314),
        false,
        false,
        false
    )]
    #[case::py314_holds_a_class_annotation_naming_an_unbound_name(
        "class Point:\n    x: Missing\n",
        Some(PythonVersion::PY314),
        false,
        false,
        false
    )]
    #[case::py314_holds_a_name_only_a_try_binds(
        "try:\n    from fast import Alias\nexcept ImportError:\n    Alias = int\n\n\ndef f(x: Alias) -> Alias:\n    return x\n",
        Some(PythonVersion::PY314),
        false,
        false,
        false
    )]
    #[case::py314_resolves_a_quoted_forward_reference(
        "def f(x: \"Missing\") -> None:\n    return None\n",
        Some(PythonVersion::PY314),
        false,
        false,
        true
    )]
    #[case::py314_holds_a_deleted_name(
        "def f(x: Alias) -> Alias:\n    return x\n\n\nAlias = int\ndel Alias\n",
        Some(PythonVersion::PY314),
        false,
        false,
        false
    )]
    #[case::constant_below_its_reader_unresolved(
        CONSTANT_BELOW_ITS_READER,
        None,
        false,
        false,
        false
    )]
    #[case::hoisted_constant_resolves_under_the_band(
        CONSTANT_BELOW_ITS_READER,
        None,
        false,
        true,
        true
    )]
    #[case::hoisted_import_resolves_under_the_band(
        "def convert(value: Sequence) -> Sequence:\n    return value\n\n\nfrom collections.abc import Sequence\n",
        None,
        false,
        true,
        true
    )]
    #[case::anchored_constant_stays_unresolved_under_the_band(
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = build()\n",
        None,
        false,
        true,
        false
    )]
    #[case::conditional_write_stays_unresolved_under_the_band(
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nif flag:\n    Alias = int\n",
        None,
        false,
        true,
        false
    )]
    #[case::definition_below_its_reader_stays_unresolved_under_the_band(
        "def visit(node: Node) -> Node:\n    return node\n\n\nclass Node:\n    pass\n",
        None,
        false,
        true,
        false
    )]
    #[case::sorted_definition_reads_unresolved_under_the_band(
        CLASS_ABOVE_ITS_READER,
        None,
        true,
        true,
        false
    )]
    #[case::declined_band_reads_the_source_order(
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = int; LIMIT = 2\n",
        None,
        false,
        true,
        false
    )]
    #[case::method_annotation_resolves_under_the_band(
        "class Converter:\n    def convert(self, value: Alias) -> Alias:\n        return value\n\n\nAlias = int\n",
        None,
        false,
        true,
        true
    )]
    #[case::one_unresolved_name_holds_under_the_band(
        "def visit(node: Node, alias: Alias) -> Node:\n    return node\n\n\nAlias = int\n\n\nclass Node:\n    pass\n",
        None,
        false,
        true,
        false
    )]
    #[case::trailing_constant_stays_unresolved_under_the_band(
        "def convert(value: Alias) -> Alias:\n    return value\n\n\nAlias = convert\n",
        None,
        false,
        true,
        false
    )]
    fn annotations_are_inert_reads_each_branch(
        #[case] src: &str,
        #[case] target: Option<PythonVersion>,
        #[case] sorts_definitions: bool,
        #[case] banded: bool,
        #[case] expected: bool,
    ) {
        let mut config = Config::default();
        config.rules.alphabetize_siblings.sort_definitions = sorts_definitions;
        config.rules.band_constants.enabled = banded;
        config.target_version = target;
        let rule = PruneInertImports::from_config(&config);
        assert_eq!(annotations_are_inert(&rule, &parse(src)), expected);
    }
}
