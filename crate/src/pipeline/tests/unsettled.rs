//! Tests over `Pipeline::unsettled`.

use super::*;

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

#[test]
fn unsettled_among_answers_empty_where_no_rule_fired() {
    let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let source = parse("x = 1\n");

    assert!(
        pipeline
            .unsettled_among(&source, &BTreeSet::new())
            .is_empty()
    );
}

#[test]
fn unsettled_among_reapplies_only_the_rules_that_fired() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(never_settles("widener")),
        Box::new(never_settles("other-widener")),
    ]);
    let source = parse("x = 1\n");
    let fired = BTreeSet::from([RuleId::from("other-widener")]);

    assert_eq!(
        pipeline.unsettled_among(&source, &fired),
        vec![RuleId::from("other-widener")],
    );
}
