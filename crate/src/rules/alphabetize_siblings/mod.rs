//! Alphabetizes sibling AST nodes wherever order does not carry
//! meaning: classes and functions in a body, class-scope field runs,
//! keyword-only parameters, call kwargs, dict keys, set elements,
//! import names and alias lists within each section, `global` /
//! `nonlocal` / `del` name lists, and the strings inside `__all__` /
//! `__slots__`. Sorting runs through the `primitives::orderer`
//! permute and assemble primitives, a recursive rewriter folding inner
//! sorts into the outer scope's replacement text so each outermost
//! scope emits one edit, or one per notebook cell. Positional-or-
//! keyword parameters never reorder and only the keyword-only block
//! past `*` sorts, a class whose header generates a field-ordered
//! constructor holds its field run, and a decorated definition holds
//! its slot at module scope while sorting inside a class body.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt, StmtAssign, StmtClassDef,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextSize};

use self::{
    docstring_entries::collect_docstring_entry_edits,
    enums::Enumerations,
    leaves::collect_leaf_edits,
    reorders::{joined_key, joined_text},
    rewrite::{RewriteCtx, body_layout, class_rows, import_gap},
};

use crate::{
    config::Config,
    primitives::{
        binding::single_name_target, imports::defers_annotations, orderer::Seatings,
        scope::BodyScope,
    },
    rules::{Rule, RuleId},
    source::Source,
};
pub(crate) use dict::sets_dividers;
pub(crate) use reorders::{Reorders, Sorted};

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

#[derive(Clone, Debug, PartialEq)]
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

    pub(crate) const PRESERVES_TREE: bool = false;

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

    /// Builds the context every body rewrite over `source` shares, with
    /// `leaf_edits` rewriting the leaves inside each member.
    fn ctx<'a>(
        &'a self,
        source: &'a Source,
        enumerations: &'a Enumerations<'a>,
        leaf_edits: &'a [Edit],
    ) -> RewriteCtx<'a> {
        RewriteCtx {
            defer_annotations: defers_annotations(&source.ast().body),
            enumerations,
            first_party: &self.first_party,
            group_imports: self.group_imports,
            group_methods: self.group_methods,
            keyword_fields_from: TextSize::default(),
            leaf_edits,
            orders_members: false,
            sort_definitions: self.sort_definitions,
            source,
        }
    }

    /// Collects the rows of every class body the rule seats other than as
    /// written, keyed by the start of the body's first statement, each slot
    /// in the order the sort seats it beside whether it opens on the line
    /// directly below the slot before it.
    pub(crate) fn seatings(&self, source: &Source) -> Seatings {
        let body = &source.ast().body;
        let enumerations = Enumerations::of(body);
        let ctx = self.ctx(source, &enumerations, &[]);
        let mut classes = Classes(Vec::new());
        classes.visit_body(body);
        classes
            .0
            .into_iter()
            .filter(|class| !class.body.is_empty())
            .filter_map(|class| Some((class.body[0].start(), class_rows(ctx, class)?)))
            .collect()
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
        let enumerations = Enumerations::of(body);
        let ctx = self.ctx(source, &enumerations, &leaf_edits);
        let layout = body_layout(ctx, body, source.module_range(), BodyScope::Module);
        layout
            .assembly
            .cell_edits(source, !layout.import_run_slots.is_empty(), |i| {
                import_gap(source, &layout.import_run_slots, i)
            })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Every class definition a walk over a module reaches, nested ones
/// included.
struct Classes<'a>(Vec<&'a StmtClassDef>);

impl<'a> StatementVisitor<'a> for Classes<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::ClassDef(class) = stmt {
            self.0.push(class);
        }
        walk_stmt(self, stmt);
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

    #[rstest]
    #[case::reseated("class K:\n    b = 1\n    a = 2\n", Some(vec![(1, false), (0, true)]))]
    #[case::sorted("class K:\n    a = 1\n    b = 2\n", None)]
    fn seatings_record_a_class_body_the_sort_seats_other_than_as_written(
        #[case] src: &str,
        #[case] expected: Option<Vec<(usize, bool)>>,
    ) {
        let source = parse(src);
        let class = source.ast().body[0]
            .as_class_def_stmt()
            .expect("the source opens on a class");
        let seatings = AlphabetizeSiblings::from_config(&Config::default()).seatings(&source);

        assert_eq!(seatings.get(&class.body[0].start()).cloned(), expected);
    }
}
