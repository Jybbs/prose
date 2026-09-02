//! Batch-surface tests over the rules a fold splices into one buffer,
//! and over the [`Sharing`] that decides which of them join.

use super::*;

#[test]
fn from_rules_seats_each_rule_beside_the_earlier_seats_it_shares_a_splice_with() {
    let seated = |slug: &'static str| -> Box<dyn Rule> {
        Box::new(GroupSentinelRule {
            groups: Vec::new(),
            id: RuleId::from(slug),
        })
    };
    // The registry declares `normalize-literals` independent of
    // `shed-backslash-continuations`, and `strip-none-return`
    // independent of both, so each seat names every earlier seat it
    // shares a splice with and none seated ahead of it.
    let pipeline = Pipeline::from_rules(vec![
        seated("shed-backslash-continuations"),
        seated("normalize-literals"),
        seated("strip-none-return"),
    ]);

    assert_eq!(pipeline.shares, [vec![], vec![0], vec![0, 1]]);
}

#[test]
fn run_batches_a_declared_pair_against_one_buffer() {
    // `strip-trailing-commas` shares a splice with `normalize-literals`
    // in the registry's table, so the second sentinel reads the buffer
    // the first read.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![
        capturing(&seen, "normalize-literals", vec![replacement("y", 0, 1)]),
        capturing(&seen, "strip-trailing-commas", vec![replacement("2", 4, 5)]),
    ]);

    let (result, _) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(result.text(), "y = 2\n");
    assert_eq!(captured(&seen), ["x = 1\n", "x = 1\n"]);
}

#[test]
fn run_batches_adjacent_edits_from_two_rules() {
    // An edit ending where the next begins is no overlap, so both rules
    // read the base buffer and land in one splice.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![
        capturing(&seen, "rewrite-head", vec![replacement("a", 0, 1)]),
        capturing(&seen, "rewrite-gap", vec![replacement("b", 1, 2)]),
    ])
    .sharing(Sharing::Always);

    let (result, _) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(result.text(), "ab= 1\n");
    assert_eq!(captured(&seen), ["x = 1\n", "x = 1\n"]);
}

/// Neither sentinel is in the registry's shared-splice column, so the
/// second reads the first's rewrite under either sharing and the batch
/// closes between them.
#[rstest]
#[case::declared(Sharing::Declared)]
#[case::never(Sharing::Never)]
fn a_batch_closing_hands_the_downstream_rule_the_upstream_rewrite(#[case] sharing: Sharing) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![
        capturing(&seen, "rewrite-x-to-y", vec![replacement("y", 0, 1)]),
        capturing(&seen, "downstream-observer", Vec::new()),
    ])
    .sharing(sharing);

    pipeline.run(parse("x = 1\n")).expect("both stages succeed");

    assert_eq!(captured(&seen), ["x = 1\n", "y = 1\n"]);
}

#[test]
fn run_batches_independent_rules_against_one_buffer() {
    // The second reads the buffer the first read rather than the
    // first's rewrite, and both rewrites land in one splice.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![
        capturing(&seen, "rewrite-x-to-y", vec![replacement("y", 0, 1)]),
        capturing(&seen, "rewrite-1-to-2", vec![replacement("2", 4, 5)]),
    ])
    .sharing(Sharing::Always);

    let (result, diagnostics) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(result.text(), "y = 2\n");
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(captured(&seen), ["x = 1\n", "x = 1\n"]);
}

#[test]
fn run_closes_a_batch_ahead_of_an_overlapping_edit() {
    // The second rule's edit covers the first's, so the batch holding
    // the first closes and the second re-reads the spliced buffer
    // before its own edit lands.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![
        capturing(&seen, "rewrite-x-to-y", vec![replacement("y", 0, 1)]),
        capturing(&seen, "rewrite-head-to-z", vec![replacement("z", 0, 1)]),
    ])
    .sharing(Sharing::Always);

    let (result, diagnostics) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(result.text(), "z = 1\n");
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(captured(&seen), ["x = 1\n", "x = 1\n", "y = 1\n"]);
}

#[test]
fn run_drops_a_rule_whose_edits_vanish_once_the_batch_closes() {
    // The second rule's edit overlaps the first's, so the batch closes,
    // and the spliced buffer no longer opens with `x`, so its re-read
    // emits nothing.
    let pipeline = Pipeline::from_rules(vec![
        Box::new(prefix_rule("rewrite-x-to-y", "x", "y")),
        Box::new(prefix_rule("rewrite-x-to-z", "x", "z")),
    ])
    .sharing(Sharing::Always);

    let (result, diagnostics) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(result.text(), "y = 1\n");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
#[should_panic(expected = "invariant: a batch whose splice is rejected")]
fn run_flags_a_replay_that_passes_where_its_batch_was_rejected() {
    // Spliced together the two rewrites demote the `__future__` import
    // and fail the gate, whereas replayed one at a time the second rule
    // sees `x = 1` and emits nothing, so the declared pair has read
    // each other's rewrite on this buffer.
    let pipeline = Pipeline::from_rules(vec![
        sentinel("normalize-literals", vec![replacement("x = 1", 0, 34)]),
        Box::new(GuardedRule {
            edit: replacement("from __future__ import division", 35, 44),
            guard: "from __future__",
            id: RuleId::from("strip-trailing-commas"),
        }),
    ]);
    let _ = pipeline.run(parse(FUTURE_LEAD));
}

#[test]
fn run_forwards_a_notebook_through_one_batched_splice() {
    let pipeline = Pipeline::from_rules(vec![
        sentinel("widen-x", vec![replacement("xx", 0, 1)]),
        sentinel("widen-y", vec![replacement("yy", 7, 8)]),
    ])
    .sharing(Sharing::Always);
    let source = notebook(&["x = 1\n", "y = 2\n"]);

    let (result, _) = pipeline.run(source).expect("notebook run succeeds");

    assert_eq!(result.text(), "xx = 1\n\nyy = 2\n\n");
    assert_eq!(result.cell_texts(), ["xx = 1\n", "yy = 2\n"]);
}

#[test]
fn run_names_every_rule_of_a_batch_the_gate_rejects_under_always() {
    // The appended assignment and the demoting rewrite splice into one
    // buffer that parses and fails to compile, which the probe's
    // sharing reports as the batch rather than replaying.
    let pipeline = Pipeline::from_rules(vec![
        sentinel(
            "append-x",
            vec![Edit::insertion(
                "x = 1\n".to_owned(),
                FUTURE_LEAD.text_len(),
            )],
        ),
        Box::new(breaks_compile()),
    ])
    .sharing(Sharing::Always);

    assert_matches!(
        pipeline.run(parse(FUTURE_LEAD)),
        Err(PipelineError::Batch { rules })
            if rules == [RuleId::from("append-x"), RuleId::from("breaks-compile")]
    );
}

#[test]
fn run_names_every_rule_of_a_batch_the_reparse_rejects_under_always() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(breaks_parse()),
        sentinel("rewrite-y-to-z", vec![replacement("z", 6, 7)]),
    ])
    .sharing(Sharing::Always);

    assert_matches!(
        pipeline.run(parse("x = 1\ny = 2\n")),
        Err(PipelineError::Batch { rules })
            if rules == [RuleId::from("breaks-parse"), RuleId::from("rewrite-y-to-z")]
    );
}

#[test]
fn run_names_the_rule_whose_output_a_declared_batch_replay_fails_to_compile() {
    // The demoting rewrite carries a slug sharing a splice with the
    // appending one, so the batch splices into one buffer that fails
    // the gate and the replay names the demoting rule alone.
    let pipeline = Pipeline::from_rules(vec![
        sentinel(
            "normalize-literals",
            vec![Edit::insertion(
                "x = 1\n".to_owned(),
                FUTURE_LEAD.text_len(),
            )],
        ),
        Box::new(GroupSentinelRule {
            groups: breaks_compile().groups,
            id: RuleId::from("strip-trailing-commas"),
        }),
    ]);

    assert_matches!(
        pipeline.run(parse(FUTURE_LEAD)),
        Err(PipelineError::Compile { rule, .. }) if rule.as_str() == "strip-trailing-commas"
    );
}

#[test]
fn run_names_the_rule_whose_splice_a_declared_batch_replay_rejects() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(GroupSentinelRule {
            groups: breaks_parse().groups,
            id: RuleId::from("normalize-literals"),
        }),
        sentinel("strip-trailing-commas", vec![replacement("z", 6, 7)]),
    ]);

    assert_matches!(
        pipeline.run(parse("x = 1\ny = 2\n")),
        Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "normalize-literals"
    );
}
