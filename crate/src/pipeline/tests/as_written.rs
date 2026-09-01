//! As-written-surface tests over the replay pass, which folds from the
//! first seat a diagnose pass found editing.

use super::*;

#[test]
fn run_as_written_leaves_the_unedited_prefix_to_the_diagnose_pass() {
    // The first rule reads the buffer and edits nothing, so the
    // diagnose pass has already answered for it and the fold opens
    // at the second. Its capture log holds the one read.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![
        capturing(&seen, "reads-only", Vec::new()),
        Box::new(GroupSentinelRule {
            groups: vec![vec![replacement("y", 0, 1)]],
            id: RuleId::from("rewrite-x-to-y"),
        }),
    ]);

    let (formatted, _) = pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(formatted.text(), "y = 1\n");
    assert_eq!(captured(&seen), ["x = 1\n"]);
}

#[test]
fn run_as_written_passes_a_clean_rewrite() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![replacement("y", 0, 1)]],
        id: RuleId::from("rewrite-x-to-y"),
    })]);
    let source = parse("x = 1\n");

    assert!(pipeline.run_as_written(source).is_ok());
}

#[test]
fn run_as_written_passes_an_overlapping_group_as_a_no_op() {
    let pipeline = Pipeline::from_rules(vec![Box::new(self_overlapping())]);
    let source = parse("x = 1\n");

    assert!(pipeline.run_as_written(source).is_ok());
}

#[test]
fn run_as_written_passes_when_no_rule_edits() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![Vec::new()],
        id: RuleId::from("emits-empty-group"),
    })]);
    let source = parse("x = 1\n");

    assert!(pipeline.run_as_written(source).is_ok());
}

#[test]
fn run_as_written_resolves_a_lint_range_against_the_original_buffer() {
    // `widen-x` moves the `1` one byte right, so the lint range the
    // rewritten buffer carries is 5..6 and the as-written one 4..5.
    let pipeline = Pipeline::from_rules(vec![
        Box::new(GroupSentinelRule {
            groups: vec![vec![replacement("yy", 0, 1)]],
            id: RuleId::from("widen-x"),
        }),
        Box::new(NeedleLintRule {
            id: RuleId::from("flag-one"),
            needle: "1",
        }),
    ]);

    let (formatted, diagnostics) = pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(formatted.text(), "yy = 1\n");
    let lint = diagnostics
        .iter()
        .find(|d| d.severity.is_lint())
        .expect("lint finding");
    assert_eq!(lint.range, range(4, 5));
}

#[test]
fn run_as_written_returns_the_diagnostics_it_replayed() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![replacement("y", 0, 1)]],
        id: RuleId::from("rewrite-x-to-y"),
    })]);

    let (_, diagnostics) = pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-to-y");
}

#[test]
fn run_as_written_short_circuits_when_file_is_suppressed() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![replacement("y", 11, 12)]],
        id: RuleId::from("rewrite-x-to-y"),
    })]);
    let source = parse("# prose: off\nx = 1\n");

    let (formatted, diagnostics) = pipeline.run_as_written(source).expect("the run succeeds");

    assert_eq!(formatted.text(), "# prose: off\nx = 1\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn run_as_written_skips_the_replay_where_no_rule_edits() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![capturing(&seen, "reads-only", Vec::new())]);

    pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(captured(&seen), ["x = 1\n"]);
}

#[test]
fn run_as_written_surfaces_uncompilable_rule_output() {
    let pipeline = Pipeline::from_rules(vec![Box::new(breaks_compile())]);
    let source = parse(FUTURE_LEAD);

    assert_matches!(
        pipeline.run_as_written(source),
        Err(PipelineError::Compile { rule, .. }) if rule.as_str() == "breaks-compile"
    );
}

#[test]
fn run_as_written_surfaces_unparseable_rule_output() {
    let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
    let source = parse("x = 1\n");

    assert_matches!(
        pipeline.run_as_written(source),
        Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "breaks-parse"
    );
}
