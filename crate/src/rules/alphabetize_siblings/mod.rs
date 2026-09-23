//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning: classes and functions in a body, class-scope field runs,
//! keyword-only parameters, call kwargs, dict keys, set elements,
//! import names and alias lists within each section, `global` /
//! `nonlocal` / `del` name lists, and the strings inside `__all__` /
//! `__slots__`. A recursive rewriter over the `primitives::orderer`
//! permute and assemble primitives folds inner sorts into the outer
//! scope's replacement, so each outermost scope emits one edit, or one
//! per notebook cell. Positional-or-keyword parameters, the field run
//! of a class whose header generates its constructor, the statements of
//! a class under a `# prose: keep` header, and a decorated definition at
//! module scope each keep their order, and a statement a suppression
//! covers keeps its slot while the rewrites inside it land as edits of
//! their own.

use ruff_diagnostics::Edit;
use ruff_python_ast::StmtAssign;
use ruff_text_size::{Ranged, TextSize};

use self::{
    docstring_entries::collect_docstring_entry_edits,
    enums::Enumerations,
    leaves::collect_leaf_edits,
    reorders::{joined_key, joined_text},
    rewrite::{RewriteCtx, body_layout, import_gap},
};
use crate::{
    config::Config,
    primitives::{binding::single_name_target, imports::defers_annotations, scope::BodyScope},
    rules::{Rule, RuleId},
    source::Source,
};

mod class_graph;
mod dict;
mod docstring_entries;
mod enums;
mod leaves;
mod members;
mod module_graph;
mod reorders;
mod rewrite;
mod section_runs;

pub(crate) use reorders::Reorders;

#[derive(Debug)]
pub(crate) struct AlphabetizeSiblings {
    code_width: usize,
    first_party: Vec<String>,
    group_imports: bool,
    group_methods: bool,
    sort_definitions: bool,
    sort_dict_keys: bool,
    sort_docstring_entries: bool,
    sort_dunder_lists: bool,
}

impl AlphabetizeSiblings {
    pub(crate) const MESSAGE: &'static str = "alphabetize these siblings";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let alphabetize_siblings = &config.rules.alphabetize_siblings;
        Self {
            code_width: config.code_width(),
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            group_methods: alphabetize_siblings.group_methods,
            sort_definitions: alphabetize_siblings.sort_definitions,
            sort_dict_keys: alphabetize_siblings.sort_dict_keys,
            sort_docstring_entries: alphabetize_siblings.sort_docstring_entries,
            sort_dunder_lists: alphabetize_siblings.sort_dunder_lists,
        }
    }
}

impl Rule for AlphabetizeSiblings {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        if body.is_empty() {
            return Vec::new();
        }
        let mut leaf_edits = collect_leaf_edits(
            source,
            self.code_width,
            self.sort_dict_keys,
            self.sort_dunder_lists,
        );
        if self.sort_docstring_entries {
            leaf_edits.extend(collect_docstring_entry_edits(source));
            leaf_edits.sort_unstable();
        }
        let suppression = source.suppression_map();
        if suppression.has_format_suppression() {
            leaf_edits.retain(|edit| !suppression.suppresses(edit, Self::SLUG));
        }
        let enumerations = Enumerations::of(body);
        let ctx = RewriteCtx {
            defer_annotations: defers_annotations(body),
            enumerations: &enumerations,
            first_party: &self.first_party,
            group_imports: self.group_imports,
            group_methods: self.group_methods,
            keeps_order: false,
            keyword_fields_from: TextSize::default(),
            leaf_edits: &leaf_edits,
            orders_members: false,
            sort_definitions: self.sort_definitions,
            source,
        };
        let layout = body_layout(ctx, body, source.module_range(), BodyScope::Module);
        let groups = layout
            .assembly
            .cell_edits(source, !layout.import_run_slots.is_empty(), |i| {
                import_gap(source, &layout.import_run_slots, i)
            });
        if layout.held.is_empty() {
            return groups;
        }
        let mut edits: Vec<Edit> = groups.into_iter().flatten().chain(layout.held).collect();
        edits.sort_unstable();
        edits
            .chunk_by(|a, b| source.same_cell(a.start(), b.start()))
            .map(<[Edit]>::to_vec)
            .collect()
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// True when `assign` binds one of the dunder lists whose elements
/// this rule sorts.
pub(super) fn dunder_list(assign: &StmtAssign) -> bool {
    matches!(single_name_target(assign), Some("__all__" | "__slots__"))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, at, first_value, parse};

    #[test]
    fn apply_skips_dict_key_reorder_when_config_disables_it() {
        let src = "row = {\"model\": 1, \"epochs\": 2}\ntags = {\"zeta\", \"alpha\"}\n";
        let mut config = Config::default();
        config.rules.alphabetize_siblings.sort_dict_keys = false;
        let rule = AlphabetizeSiblings::from_config(&config);
        let source = parse(src);
        let edits = rule.apply(&source).into_iter().flatten().collect();
        assert_eq!(
            applied_text(&source, edits),
            "row = {\"model\": 1, \"epochs\": 2}\ntags = {\"alpha\", \"zeta\"}\n",
        );
    }

    #[test]
    fn apply_skips_docstring_entry_reorder_when_config_disables_it() {
        let src = indoc! {"
            def f():
                \"\"\"Summary.

                Args:
                    bar: two
                    alpha: one
                \"\"\"
                pass
        "};
        let mut config = Config::default();
        config.rules.alphabetize_siblings.sort_docstring_entries = false;
        let rule = AlphabetizeSiblings::from_config(&config);
        let source = parse(src);
        let edits = rule.apply(&source).into_iter().flatten().collect();
        let text = applied_text(&source, edits);
        let args_section = &text[..at(&text, "\"\"\"\n    pass").start().to_usize()];
        assert!(
            at(args_section, "bar: two").start() < at(args_section, "alpha: one").start(),
            "docstring entries should keep source order when sort-docstring-entries is off",
        );
    }

    #[rstest]
    #[case::packed_commented_list("x = [b, a,  # c\n     d]\n", true)]
    #[case::packed_commented_tuple("x = (b, a,  # c\n     d)\n", true)]
    #[case::packed_commented_set("x = {b, a,  # c\n     d}\n", true)]
    #[case::one_per_line_commented("x = [\n    b,  # c\n    a,\n]\n", false)]
    #[case::single_element("x = [b]\n", false)]
    #[case::packed_commented_dict("x = {\"b\": 1, \"a\": 2,  # c\n     \"d\": 3}\n", true)]
    #[case::single_entry_dict("x = {\"b\": 1}\n", false)]
    fn holds_as_laid_out_reads_the_layout_gates(#[case] src: &str, #[case] expected: bool) {
        let source = parse(src);
        let reorders = Reorders::from_config(&Config::default());
        assert_eq!(
            reorders.holds_as_laid_out(&source, first_value(&source).into()),
            expected
        );
    }
}
