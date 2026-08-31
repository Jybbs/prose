//! Settle-surface tests over what a completed run still leaves behind.

use super::*;

#[test]
fn settle_report_holds_the_first_editing_rule_as_its_witness() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(never_settles("first")),
        Box::new(never_settles("second")),
    ]);
    let source = parse("x = 1\n");

    let report = pipeline.settle_report(&source);

    assert_eq!(
        report.editing,
        vec![RuleId::from("first"), RuleId::from("second")]
    );
    assert_matches!(report.witness, Some((id, _)) if id == RuleId::from("first"));
}

#[test]
fn settle_report_names_a_rule_whose_fix_never_lands() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(self_overlapping()),
        Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::range_replacement("x".to_owned(), range(0, 1))]],
            id: RuleId::from("rewrite-x-to-x"),
        }),
    ]);
    let source = parse("x = 1\n");

    let report = pipeline.settle_report(&source);

    assert!(report.editing.is_empty());
    assert_eq!(
        report.unlanded,
        vec![self_overlapping().id(), RuleId::from("rewrite-x-to-x")]
    );
    assert!(report.witness.is_none());
}

#[test]
fn settle_report_names_the_editing_rule_and_the_text_it_weaves() {
    let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let source = parse("x = 1\n");

    let report = pipeline.settle_report(&source);

    assert_eq!(report.editing, vec![RuleId::from("widener")]);
    assert!(report.unlanded.is_empty());
    assert_matches!(
        report.witness,
        Some((id, text)) if id == RuleId::from("widener") && text == "yy = 1\n"
    );
}

#[test]
fn unsettled_answers_empty_under_a_file_level_suppression() {
    let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let source = parse("# prose: off\nx = 1\n");

    assert!(pipeline.unsettled(&source).is_empty());
}

#[test]
fn unsettled_names_a_rule_still_editing_a_notebook() {
    let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let source = notebook(&["x = 1\n", "y = 2\n"]);

    assert_eq!(pipeline.unsettled(&source), vec![RuleId::from("widener")]);
}

#[test]
fn unsettled_names_only_the_rules_whose_edits_would_rewrite() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(never_settles("widener")),
        Box::new(GroupSentinelRule {
            groups: Vec::new(),
            id: RuleId::from("emits-nothing"),
        }),
        Box::new(self_overlapping()),
    ]);
    let source = parse("x = 1\n");

    assert_eq!(
        pipeline.unsettled(&source),
        vec![RuleId::from("widener")],
        "an empty group and an unspliceable one both leave the source settled",
    );
}

#[test]
fn unsettled_reads_the_subset_the_pipeline_carries() {
    let source = parse("x = 1\n");
    let carried = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let bare = Pipeline::empty();

    assert_eq!(carried.unsettled(&source), vec![RuleId::from("widener")]);
    assert!(bare.unsettled(&source).is_empty());
}

#[test]
fn unsettled_skips_a_rule_whose_edits_fall_in_a_suppressed_block() {
    let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let source = parse("# prose: off\nx = 1\n# prose: on\n");

    assert!(pipeline.unsettled(&source).is_empty());
}

#[test]
fn unsettled_skips_a_rule_whose_edits_splice_back_to_the_same_text() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement("x".to_owned(), range(0, 1))]],
        id: RuleId::from("rewrite-x-to-x"),
    })]);
    let source = parse("x = 1\n");

    assert!(pipeline.unsettled(&source).is_empty());
}
