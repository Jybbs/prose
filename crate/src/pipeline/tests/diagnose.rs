//! Diagnose-surface tests, the findings collected against the buffer as written.

use super::*;

#[test]
fn diagnose_collects_against_the_original_buffer_without_rewriting() {
    // The first rule would rewrite `x` to `y`, the second lints the
    // original `x` at 0..1. `diagnose` must not apply the first
    // rule's edit, so the lint range stays valid against the
    // untouched buffer and both findings surface together.
    let pipeline = Pipeline::from_rules(vec![
        sentinel("rewrite-x-to-y", vec![replacement("y", 0, 1)]),
        Box::new(LintSentinelRule {
            id: RuleId::from("flag-x"),
            ranges: vec![range(0, 1)],
        }),
    ]);
    let source = parse("x = 1\n");

    let diagnostics = pipeline.diagnose(&source);

    assert_eq!(diagnostics.len(), 2);
    let format = diagnostics
        .iter()
        .find(|d| d.severity.is_format())
        .expect("format finding");
    assert_eq!(format.rule.as_str(), "rewrite-x-to-y");
    let lint = diagnostics
        .iter()
        .find(|d| d.severity.is_lint())
        .expect("lint finding");
    assert_eq!(lint.rule.as_str(), "flag-x");
    assert_eq!(lint.range, range(0, 1));
}

#[test]
fn diagnose_drops_a_lint_under_a_per_line_ignore_directive() {
    // A bare `# prose: ignore` suppresses every rule on its line, so
    // the lint at `x` (line 1) is dropped through diagnose's
    // lint-suppression tail rather than its file-level short-circuit.
    let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
        id: RuleId::from("flag-x"),
        ranges: vec![range(0, 1)],
    })]);
    let source = parse("x = 1  # prose: ignore\n");

    assert!(pipeline.diagnose(&source).is_empty());
}

#[test]
fn diagnose_drops_a_whole_group_holding_one_suppressed_edit() {
    // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
    //         |0--------|11----|17--------|27----|33
    // The group bundles an edit at 11..16 (inside the suppressed
    // [0..17) span) with one at 27..32. The group drops as a unit,
    // so diagnose emits nothing.
    let pipeline = Pipeline::from_rules(vec![sentinel(
        "rewrite-x-and-z",
        vec![replacement("y", 11, 16), replacement("Z", 27, 32)],
    )]);
    let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

    assert!(pipeline.diagnose(&source).is_empty());
}

#[test]
fn diagnose_drops_findings_under_a_suppressed_span() {
    let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
        id: RuleId::from("flag-stuff"),
        ranges: vec![range(13, 14)],
    })]);
    let source = parse("# prose: off\nx = 1\n");

    assert!(pipeline.diagnose(&source).is_empty());
}

#[test]
fn diagnosed_names_no_seat_where_no_rule_edits() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(LintSentinelRule {
            id: RuleId::from("flag-x"),
            ranges: vec![range(0, 1)],
        }),
        Box::new(GroupSentinelRule {
            groups: vec![Vec::new()],
            id: RuleId::from("emits-empty-group"),
        }),
    ]);

    let (diagnostics, edits_at) = pipeline.diagnosed(&parse("x = 1\n"));

    assert_eq!(edits_at, None);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn diagnosed_stops_at_the_first_rule_holding_a_fix_group() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(GroupSentinelRule {
            groups: vec![Vec::new()],
            id: RuleId::from("emits-empty-group"),
        }),
        sentinel("rewrite-x-to-y", vec![replacement("y", 0, 1)]),
        sentinel("rewrite-1-to-2", vec![replacement("2", 4, 5)]),
    ]);

    let (_, edits_at) = pipeline.diagnosed(&parse("x = 1\n"));

    assert_eq!(edits_at, Some(1));
}
