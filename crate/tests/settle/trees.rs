//! Holds every rule declaring `PRESERVES_TREE` to the tree its input
//! parses to. The check files every rewrite the subset sweep records for
//! such a rule, pair runs included, whose output's `ComparableModModule`
//! differs from its input's, and runs every such rule together in one
//! pipeline held to the same comparison, a rejection there filing too.
//! Over the fixture tree it also files a rule declaring `false` whose
//! every rewrite keeps the tree.

use std::{iter::successors, ops::RangeInclusive};

use indoc::indoc;
use ruff_python_ast::{
    Stmt,
    comparable::{ComparableModModule, ComparableStmt},
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_text_size::Ranged;

use super::*;
use crate::common::excerpt;

/// Every statement a walk reaches, each ahead of the statements nested
/// in it.
#[derive(Default)]
struct Statements<'a>(Vec<&'a Stmt>);

impl<'a> StatementVisitor<'a> for Statements<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.0.push(stmt);
        walk_stmt(self, stmt);
    }
}

/// Files each changed run whose rule declares `PRESERVES_TREE` and whose
/// output parses to another tree, then the run of [`Probes::joint`],
/// filing one of the memo's first `solo` runs only where this sweep
/// reports its rule. Over the fixture tree it records whether each
/// `false` rule's runs changed the tree.
pub(super) fn check_trees(
    probes: &Probes,
    memo: &Memo,
    solo: usize,
    source: &Source,
    path: &Path,
    findings: &mut Findings,
) {
    let checked = memo
        .runs
        .iter()
        .enumerate()
        .filter_map(|(at, ((seat, input), applied))| {
            let Applied::Changed(output) = applied else {
                return None;
            };
            let rule = probes.singles[*seat].rule;
            let preserves = preserves_tree(rule.as_str());
            let reported = probes.reported.contains(&rule);
            let checks = if preserves {
                at >= solo || reported
            } else {
                probes.fixtures && reported
            };
            checks.then_some((input, (rule, preserves, output)))
        })
        .into_group_map();
    for (input, runs) in checked {
        let before = parsed(input);
        let tree = ComparableModModule::from(before.ast());
        for (rule, preserves, output) in runs {
            let after = parsed(output);
            let reshaped = ComparableModModule::from(after.ast()) != tree;
            if !preserves {
                *findings.changing.entry(rule).or_default() |= reshaped;
            } else if reshaped {
                findings.reshaped.record_hit(
                    format!("`{rule}` changes the tree it rewrites {}", probes.budget),
                    path,
                    Hit {
                        detail: Some(reshaping(&before, &after, &format!("after `{rule}`"))),
                        ..probes.hit(path, &[rule])
                    },
                );
            }
        }
    }
    let Some(joint) = &probes.joint else {
        return;
    };
    let together = format!(
        "every rule declaring `PRESERVES_TREE` together {}",
        probes.budget
    );
    let (defect, detail) = match joint.format(source.clone()) {
        Err(error) => (format!("{together} was rejected: {error}"), None),
        Ok(after)
            if ComparableModModule::from(after.ast())
                != ComparableModModule::from(source.ast()) =>
        {
            (
                format!("{together} changes the tree it rewrites"),
                Some(reshaping(source, &after, "after every rule together")),
            )
        }
        Ok(_) => return,
    };
    findings.reshaped.record_hit(
        defect,
        path,
        Hit {
            detail,
            ..probes.hit(path, &[])
        },
    );
}

/// Returns the zero-based rows of `before` holding the innermost statement
/// whose tree `after` changes, or `None` where both statement walks agree
/// on every statement `before` holds.
fn moved_rows(before: &Source, after: &Source) -> Option<RangeInclusive<usize>> {
    let (old, new) = (statements(before), statements(after));
    let differs = |at: usize| {
        new.get(at)
            .is_none_or(|stmt| ComparableStmt::from(old[at]) != ComparableStmt::from(*stmt))
    };
    let at = successors((0..old.len()).find(|&at| differs(at)), |&at| {
        let within = old[at].range();
        (at + 1..old.len())
            .take_while(|&inner| within.contains_range(old[inner].range()))
            .find(|&inner| differs(inner))
    })
    .last()?;
    let index = before.source_file().index();
    let row = |offset| index.line_index(offset).to_zero_indexed();
    Some(row(old[at].start())..=row(old[at].end()))
}

/// Parses a buffer a single-rule run read or wrote, which the pipeline
/// has already parsed once.
fn parsed(text: &str) -> Source {
    text.parse()
        .expect("invariant: a buffer a single-rule run read or wrote parses")
}

/// Renders the excerpt from `before` to `after`, headed `"before"` and
/// `to`, at the innermost statement whose tree changed, or at the diff's
/// first hunk where no statement holds the change or no hunk reaches its
/// rows.
fn reshaping(before: &Source, after: &Source, to: &str) -> String {
    moved_rows(before, after)
        .map(|rows| excerpt("before", to, before.text(), after.text(), rows))
        .filter(|shown| !shown.is_empty())
        .unwrap_or_else(|| excerpt("before", to, before.text(), after.text(), ..))
}

/// Collects every statement `source` holds, each ahead of the statements
/// nested in it.
fn statements(source: &Source) -> Vec<&Stmt> {
    let mut walk = Statements::default();
    walk.visit_body(&source.ast().body);
    walk.0
}

#[test]
fn check_trees_files_a_joint_run_that_changes_the_tree() {
    let mut probes = Probes::build(88);
    probes.joint = Some(Pipeline::with_filters(
        &Config::default(),
        &[rule("strip-none-return")],
        &[],
    ));
    let memo = Memo {
        probes: &probes,
        runs: IndexMap::default(),
    };
    let mut findings = Findings::default();
    let source = parsed("def f() -> None:\n    pass\n");

    check_trees(
        &probes,
        &memo,
        0,
        &source,
        Path::new("case.py"),
        &mut findings,
    );

    let rendered = findings.reshaped.render("reshaped");
    assert!(
        rendered.contains("every rule declaring `PRESERVES_TREE`"),
        "{rendered}"
    );
    assert!(
        rendered.contains("changes the tree it rewrites"),
        "{rendered}"
    );
}

#[rstest]
#[case::a_pair_run_whatever_its_rule("align-equals", 0, &[], true, 1, None)]
#[case::a_solo_run_of_an_unreported_rule("align-equals", 1, &[], true, 0, None)]
#[case::a_changing_rule_off_the_fixture_tree("reflow-calls", 1, &["reflow-calls"], false, 0, None)]
fn check_trees_files_a_run_in_the_sweep_that_reports_it(
    #[case] slug: &str,
    #[case] solo: usize,
    #[case] reported: &[&str],
    #[case] fixtures: bool,
    #[case] reshaped: usize,
    #[case] changing: Option<bool>,
) {
    let mut probes = Probes::build(88);
    probes.fixtures = fixtures;
    probes.joint = None;
    probes.reported = reported.iter().copied().map(rule).collect();
    let rule = rule(slug);
    let memo = Memo {
        probes: &probes,
        runs: IndexMap::from_iter([(
            (probes.solo[&rule], Rc::from("x = 1\n")),
            Applied::Changed(Rc::from("x = 2\n")),
        )]),
    };
    let mut findings = Findings::default();

    check_trees(
        &probes,
        &memo,
        solo,
        &parsed("x = 1\n"),
        Path::new("case.py"),
        &mut findings,
    );

    assert_eq!(findings.reshaped.len(), reshaped);
    assert_eq!(findings.changing.get(&rule).copied(), changing);
}

#[rstest]
#[case::a_kept_tree_under_a_preserving_rule("align-equals", "x=1\n", "x = 1\n", 0, None)]
#[case::a_changed_tree_under_a_preserving_rule("align-equals", "x = 1\n", "x = 2\n", 1, None)]
#[case::a_kept_tree_under_a_changing_rule("reflow-calls", "x=1\n", "x = 1\n", 0, Some(false))]
#[case::a_changed_tree_under_a_changing_rule("reflow-calls", "x = 1\n", "x = 2\n", 0, Some(true))]
fn check_trees_holds_each_run_to_its_rules_declaration(
    #[case] slug: &str,
    #[case] input: &str,
    #[case] output: &str,
    #[case] reshaped: usize,
    #[case] changing: Option<bool>,
) {
    let rule = rule(slug);
    let mut probes = Probes::build(88);
    probes.fixtures = true;
    probes.reported = BTreeSet::from([rule]);
    let memo = Memo {
        probes: &probes,
        runs: IndexMap::from_iter([(
            (probes.solo[&rule], Rc::from(input)),
            Applied::Changed(Rc::from(output)),
        )]),
    };
    let mut findings = Findings::default();

    check_trees(
        &probes,
        &memo,
        1,
        &parsed(input),
        Path::new("case.py"),
        &mut findings,
    );

    assert_eq!(findings.reshaped.len(), reshaped);
    assert_eq!(findings.changing.get(&rule).copied(), changing);
}

#[test]
fn moved_rows_is_none_where_every_statement_before_holds_agrees() {
    assert_eq!(
        moved_rows(&parsed("x = 1\n"), &parsed("x = 1\ny = 2\n")),
        None
    );
}

#[rstest]
#[case::a_nested_body(
    indoc! {"
        class C:
            def f(self):
                return 1

            def g(self):
                return 2
    "},
    indoc! {"
        class C:
            def f(self):
                return 1

            def g(self):
                return 3
    "},
    5..=5,
)]
#[case::a_header_over_an_unchanged_body(
    "def f(a):\n    return a\n",
    "def f(b):\n    return a\n",
    0..=1,
)]
#[case::a_dropped_trailing_statement("x = 1\ny = 2\n", "x = 1\n", 1..=1)]
fn moved_rows_names_the_innermost_statement_whose_tree_changes(
    #[case] before: &str,
    #[case] after: &str,
    #[case] rows: RangeInclusive<usize>,
) {
    assert_eq!(moved_rows(&parsed(before), &parsed(after)), Some(rows));
}

#[rstest]
#[case::at_the_statement_whose_tree_changed(
    format!("x=1\n{}y = 2\n", "a = 0\n".repeat(8)),
    format!("x = 1\n{}y = 5\n", "a = 0\n".repeat(8)),
    "-y = 2\n+y = 5",
    Some("+x = 1"),
)]
#[case::at_the_first_hunk_where_no_statement_holds_it(
    "x = 1\n".to_owned(),
    "x = 1\ny = 2\n".to_owned(),
    "+y = 2",
    None
)]
fn reshaping_shows_the_hunk_that_changed_the_tree(
    #[case] before: String,
    #[case] after: String,
    #[case] shown: &str,
    #[case] skipped: Option<&str>,
) {
    let excerpt = reshaping(&parsed(&before), &parsed(&after), "after");

    assert!(excerpt.contains(shown), "{excerpt}");
    assert!(
        skipped.is_none_or(|hunk| !excerpt.contains(hunk)),
        "{excerpt}"
    );
}
