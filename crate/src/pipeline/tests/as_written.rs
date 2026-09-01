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
        Box::new(TextCapturingRule {
            edits: Vec::new(),
            id: RuleId::from("reads-only"),
            seen: Arc::clone(&seen),
        }),
        Box::new(rewrites_x_to_y()),
    ]);

    let (formatted, _, _) = pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(formatted.text(), "y = 1\n");
    assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n"]);
}

#[test]
fn run_as_written_names_the_rules_the_fold_fired_rather_than_the_as_written_ones() {
    let rules = || -> Vec<Box<dyn Rule>> {
        vec![
            Box::new(PrefixRule {
                id: RuleId::from("settles-once"),
                reads: "x",
                writes: "q",
            }),
            Box::new(PrefixRule {
                id: RuleId::from("widens-downstream"),
                reads: "q",
                writes: "qq",
            }),
        ]
    };
    let both = BTreeSet::from([
        RuleId::from("settles-once"),
        RuleId::from("widens-downstream"),
    ]);

    let (formatted, diagnostics, fired) = Pipeline::from_rules(rules())
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");
    let (_, run_diagnostics) = Pipeline::from_rules(rules())
        .run(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(formatted.text(), "qq = 1\n");
    assert_eq!(
        fired_rules(&diagnostics),
        BTreeSet::from([RuleId::from("settles-once")]),
        "the as-written diagnostics name only the rule that edits the buffer as written",
    );
    assert_eq!(fired, both);
    assert_eq!(fired_rules(&run_diagnostics), both);
}

#[test]
fn run_as_written_passes_a_clean_rewrite() {
    let pipeline = Pipeline::from_rules(vec![Box::new(rewrites_x_to_y())]);
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
        Box::new(never_settles("widen-x")),
        Box::new(NeedleLintRule {
            id: RuleId::from("flag-one"),
            needle: "1",
        }),
    ]);

    let (formatted, diagnostics, _) = pipeline
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
    let pipeline = Pipeline::from_rules(vec![Box::new(rewrites_x_to_y())]);

    let (_, diagnostics, _) = pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-to-y");
}

#[test]
fn run_as_written_short_circuits_when_file_is_suppressed() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement("y".to_owned(), range(11, 12))]],
        id: RuleId::from("rewrite-x-to-y"),
    })]);
    let source = parse("# prose: off\nx = 1\n");

    let (formatted, diagnostics, _) = pipeline.run_as_written(source).expect("the run succeeds");

    assert_eq!(formatted.text(), "# prose: off\nx = 1\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn run_as_written_skips_the_replay_where_no_rule_edits() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![Box::new(TextCapturingRule {
        edits: Vec::new(),
        id: RuleId::from("reads-only"),
        seen: Arc::clone(&seen),
    })]);

    pipeline
        .run_as_written(parse("x = 1\n"))
        .expect("the run succeeds");

    assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n"]);
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
