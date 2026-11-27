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
use ruff_python_ast::{
    ExprCall, PythonVersion, Stmt, StmtAssign,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::{Ranged, TextSize};

use self::{
    docstring_entries::collect_docstring_entry_edits,
    enums::Enumerations,
    leaves::collect_leaf_edits,
    reorders::{joined_key, joined_text},
    rewrite::{RewriteCtx, body_layout, class_seatings},
};

use crate::{
    config::Config,
    primitives::{
        binding::single_name_target, orderer::Seatings, scope::BodyScope, tiering::Annotations,
    },
    rules::{Rule, RuleId},
    source::Source,
};
pub(crate) use dict::sets_dividers;
pub(crate) use lookups::lookup_dicts;
pub(crate) use reorders::{Reorders, Sorted};

mod class_graph;
mod dict;
mod docstring_entries;
mod enums;
mod leaves;
mod lookups;
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
    target_version: Option<PythonVersion>,
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
            target_version: config.target_version,
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
            annotations: Annotations::of(&source.ast().body, self.target_version),
            enumerations,
            first_party: &self.first_party,
            group_imports: self.group_imports,
            group_methods: self.group_methods,
            keeps_order: false,
            keyword_fields_from: TextSize::default(),
            leaf_edits,
            orders_members: false,
            sort_definitions: self.sort_definitions,
            source,
        }
    }

    /// Collects the rows of every class body, and of every compound arm
    /// inside one, that the rule seats other than as written, keyed by
    /// the start of the body's first statement, each slot in the order
    /// the sort seats it beside whether it opens on the line directly
    /// below the slot before it.
    pub(crate) fn seatings(&self, source: &Source) -> Seatings {
        let body = &source.ast().body;
        let enumerations = Enumerations::of(body);
        let ctx = self.ctx(source, &enumerations, &[]);
        let mut classes = Classes(Vec::new());
        classes.visit_body(body);
        let mut seatings = Seatings::default();
        for class in classes.0 {
            class_seatings(ctx, class, &mut seatings);
        }
        seatings
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
        let ctx = self.ctx(source, &enumerations, &leaf_edits);
        let layout = body_layout(ctx, body, source.module_range(), BodyScope::Module);
        let groups = layout.cell_edits(source);
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

/// Every class definition a walk over a module reaches, nested ones
/// included.
struct Classes<'a>(Vec<&'a Stmt>);

impl<'a> StatementVisitor<'a> for Classes<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if stmt.is_class_def_stmt() {
            self.0.push(stmt);
        }
        walk_stmt(self, stmt);
    }
}

/// True when `call` builds a mapping from its keywords, a `dict(...)` or
/// `frozendict(...)` call, whose keyword order is the mapping's
/// iteration order and so holds as written.
pub(super) fn builds_mapping(call: &ExprCall) -> bool {
    call.func
        .as_name_expr()
        .is_some_and(|name| matches!(name.id.as_str(), "dict" | "frozendict"))
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
    #[case::plain("def f():\n    b = 1\n    a = 2\n", "def f():\n    b = 1\n    a = 2\n")]
    #[case::import_run(
        "def f():\n    import b\n    import a\n",
        "def f():\n    import a\n    import b\n"
    )]
    #[case::nested_function_import_run(
        "def f():\n    def g():\n        from m import b, a\n",
        "def f():\n    def g():\n        from m import a, b\n"
    )]
    #[case::nested_class(
        "def f():\n    class K:\n        b = 1\n        a = 2\n",
        "def f():\n    class K:\n        a = 2\n        b = 1\n"
    )]
    #[case::leaf_edit(
        "def f():\n    tags = {\"zeta\", \"alpha\"}\n",
        "def f():\n    tags = {\"alpha\", \"zeta\"}\n"
    )]
    fn apply_returns_a_function_as_written_only_where_nothing_inside_reorders(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let rule = AlphabetizeSiblings::from_config(&Config::default());
        let edits = rule.apply(&source).into_iter().flatten().collect();
        assert_eq!(applied_text(&source, edits), expected);
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
    #[case::arm(
        "class K:\n    if X:\n        b = 1\n        a = 2\n",
        "b = 1",
        Some(vec![(1, false), (0, true)]),
    )]
    #[case::kept("class K:  # prose: keep\n    b = 1\n    a = 2\n", "b = 1", None)]
    #[case::nested(
        "class Outer:\n    class K:\n        b = 1\n        a = 2\n",
        "b = 1",
        Some(vec![(1, false), (0, true)]),
    )]
    #[case::pinned(
        "class K:\n    c = 1  # prose: skip[alphabetize-siblings]\n    b = 2\n    a = 3\n",
        "c = 1",
        Some(vec![(0, false), (2, true), (1, true)]),
    )]
    #[case::reseated("class K:\n    b = 1\n    a = 2\n", "b = 1", Some(vec![(1, false), (0, true)]))]
    #[case::sorted("class K:\n    a = 1\n    b = 2\n", "a = 1", None)]
    fn seatings_record_a_class_body_the_sort_seats_other_than_as_written(
        #[case] src: &str,
        #[case] first: &str,
        #[case] expected: Option<Vec<(usize, bool)>>,
    ) {
        let source = parse(src);
        let seatings = AlphabetizeSiblings::from_config(&Config::default()).seatings(&source);

        assert_eq!(
            seatings.get(&at(source.text(), first).start()).cloned(),
            expected
        );
    }
}
