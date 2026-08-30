//! The gate deciding whether `from __future__ import annotations` is
//! inert. It clears when the target version defers annotation
//! evaluation per PEP 749, or when every name every annotation reads is
//! bound ahead of that annotation, which a module carrying no
//! annotation at all satisfies with nothing to check.

use ruff_python_ast::{Expr, PythonVersion, Stmt, helpers::any_over_expr};
use ruff_text_size::{Ranged, TextSize};

use super::PruneInertImports;
use crate::{
    primitives::{
        binding::{BindingAnalysis, BindingKind},
        slot_holding,
        slots::slot_positions,
        walk::for_each_annotation,
    },
    rules::band_constants::BandConstants,
    source::Source,
};

/// What settles whether a name is bound ahead of the annotation reading
/// it: the binding table, the definition sort, and the seat
/// `band-constants` gives each module-body statement once the directive
/// is gone, the source order where that rule is off or declines the
/// body. A binding the band seats ahead of its reader counts as bound
/// whatever its offset, whereas a name a module-level definition binds
/// reads as unbound while `alphabetize-siblings` sorts definitions.
struct Resolution<'a> {
    analysis: &'a BindingAnalysis,
    body: &'a [Stmt],
    seats: Vec<usize>,
    sorts_definitions: bool,
}

impl<'a> Resolution<'a> {
    fn of(source: &'a Source, bands: Option<&BandConstants>, sorts_definitions: bool) -> Self {
        let body = &source.ast().body;
        Self {
            analysis: source.binding_analysis(),
            body,
            seats: bands
                .and_then(|rule| rule.forecast(source, body, source.module_range(), false))
                .map_or_else(
                    || (0..body.len()).collect(),
                    |bands| slot_positions(&bands.order),
                ),
            sorts_definitions,
        }
    }

    /// True when an unconditional module-scope write of `name` precedes
    /// the read at `offset` in the statement at `reader`, as written or
    /// as seated.
    fn binds_ahead(&self, name: &str, reader: usize, offset: TextSize) -> bool {
        if self.sorts_definitions && binds_a_definition(self.analysis, name) {
            return false;
        }
        self.analysis
            .first_unconditional_write(name)
            .is_some_and(|write| {
                write < offset || self.seats[slot_of(self.body, write)] < self.seats[reader]
            })
    }

    /// True when every annotation in the body loads only names bound
    /// ahead of it.
    fn holds_every_annotation(&self) -> bool {
        let mut resolved = true;
        for_each_annotation(self.body, |annotation| {
            resolved &= self.resolves(annotation);
        });
        resolved
    }

    /// True when every name `annotation` loads is bound ahead of it.
    fn resolves(&self, annotation: &Expr) -> bool {
        let reader = slot_of(self.body, annotation.start());
        !any_over_expr(annotation, &|expr: &Expr| {
            expr.as_name_expr().is_some_and(|name| {
                name.ctx.is_load()
                    && !self.binds_ahead(name.id.as_str(), reader, name.range.start())
            })
        })
    }
}

/// True when removing the `annotations` directive leaves every
/// annotation in `source` evaluating as it did, a binding `rule`'s band
/// forecast seats ahead of its reader resolving and a definition's
/// binding reading as unresolved while the rule sorts definitions.
pub(super) fn annotations_are_inert(rule: &PruneInertImports, source: &Source) -> bool {
    rule.target_version
        .is_some_and(PythonVersion::defers_annotations)
        || Resolution::of(source, rule.folds.bands(), rule.sorts_definitions)
            .holds_every_annotation()
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
