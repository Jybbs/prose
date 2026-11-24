//! Trip-surface tests over a run repeating its rules until none edits.

use super::*;

#[test]
fn a_first_trip_editing_nothing_ends_the_run() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let pipeline = Pipeline::from_rules(vec![capturing(&seen, "reads-only", Vec::new())]);

    let (_, _, trips) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(captured(&seen), ["x = 1\n"]);
    assert!(!trips.capped);
    assert!(trips.fired.is_empty());
}

#[test]
fn a_later_trip_opens_at_the_first_rule_past_an_as_written_run() {
    // The diagnose pass finds the second rule editing first, and the
    // first rule edits only what that rule wrote.
    let pipeline = stepping(2);

    let (formatted, diagnostics, trips) = pipeline
        .run_as_written(parse("a = 1\n"))
        .expect("the run succeeds");

    assert_eq!(formatted.text(), "c = 1\n");
    assert_eq!(
        fired_rules(&diagnostics),
        BTreeSet::from([RuleId::from("a-to-b")])
    );
    assert_eq!(
        trips.fired,
        BTreeSet::from(["a-to-b", "b-to-c"].map(RuleId::from))
    );
}

#[test]
fn a_notebook_holds_its_cells_across_trips() {
    let (formatted, _, _) = stepping(3)
        .run(notebook(&["a = 1\n", "y = 2\n"]))
        .expect("the run succeeds");

    assert_eq!(formatted.cell_texts(), ["d = 1\n", "y = 2\n"]);
}

#[test]
fn a_rejected_later_trip_names_its_rule() {
    // The second trip's rewrite demotes the `__future__` import the first
    // trip wrote.
    let pipeline = Pipeline::from_rules(vec![
        Box::new(GuardedRule {
            edit: replacement(
                "import os\nfrom __future__ import annotations\n",
                0,
                FUTURE_LEAD.text_len().into(),
            ),
            guard: "from",
            id: RuleId::from("demotes-the-import"),
        }),
        Box::new(GuardedRule {
            edit: Edit::range_deletion(range(0, 1)),
            guard: "\n",
            id: RuleId::from("strips-the-lead"),
        }),
    ]);

    assert_matches!(
        pipeline.run(parse(&format!("\n{FUTURE_LEAD}"))),
        Err(PipelineError::Compile { rule, .. }) if rule.as_str() == "demotes-the-import"
    );
}

#[test]
fn a_run_collects_every_trips_diagnostics() {
    let (_, diagnostics, _) = stepping(3).run(parse("a = 1\n")).expect("the run succeeds");

    assert_eq!(
        diagnostics.iter().map(|d| d.rule.as_str()).collect_vec(),
        ["a-to-b", "b-to-c", "c-to-d"],
    );
}

#[rstest]
#[case::one_trip_under_the_cap(7, "h = 1\n", false)]
#[case::the_last_trip_allowed(8, "i = 1\n", true)]
#[case::one_trip_past_the_cap(9, "i = 1\n", true)]
fn a_run_takes_eight_editing_trips_at_most(
    #[case] steps: usize,
    #[case] expected: &str,
    #[case] capped: bool,
) {
    let pipeline = stepping(steps);

    let (formatted, _, trips) = pipeline.run(parse("a = 1\n")).expect("the run succeeds");
    let (as_written, _, as_written_trips) = pipeline
        .run_as_written(parse("a = 1\n"))
        .expect("the run succeeds");
    let bare = pipeline.format(parse("a = 1\n")).expect("the run succeeds");

    assert_eq!(formatted.text(), expected);
    assert_eq!(as_written.text(), expected);
    assert_eq!(bare.text(), expected);
    assert_eq!(trips.capped, capped);
    assert_eq!(as_written_trips.capped, capped);
    assert_eq!(pipeline.unsettled(&formatted).is_empty(), steps < 9);
}

#[test]
fn a_trip_whose_edits_cancel_ends_the_run() {
    let pipeline = Pipeline::from_rules(vec![
        Box::new(prefix_rule("x-to-y", "x", "y")),
        Box::new(prefix_rule("y-to-x", "y", "x")),
    ]);

    let (formatted, diagnostics, trips) = pipeline.run(parse("x = 1\n")).expect("the run succeeds");

    assert_eq!(formatted.text(), "x = 1\n");
    assert_eq!(diagnostics.len(), 2);
    assert!(!trips.capped);
}

#[test]
fn trips_sets_how_many_a_run_takes() {
    let (formatted, _, trips) = stepping(3)
        .trips(2)
        .run(parse("a = 1\n"))
        .expect("the run succeeds");

    assert_eq!(formatted.text(), "c = 1\n");
    assert!(trips.capped);
}

/// Each source settles in one run whose first trip leaves a row a later
/// one joins.
#[rstest]
#[case::a_chain_on_a_joined_comprehension(
    "value = [x.name\n for x in items].a(key).b(alpha).c(beta).d(e).f(g).h(i)\n",
    2
)]
#[case::a_pair_around_a_joined_call("third = (\n    compute(a,\n        b)\n)\n", 2)]
#[case::a_pair_ahead_of_a_joined_statement(
    "value = (\n    compute(a,\n        b)\n); other = 1\n",
    2
)]
fn the_default_rules_settle_a_later_join_in_one_run(#[case] src: &str, #[case] editing: usize) {
    let pipeline = Pipeline::with_defaults(&Config::default());

    let (settled, _, trips) = pipeline.run(parse(src)).expect("the run succeeds");

    let mut trip = parse(src);
    for _ in 0..editing {
        let before = trip.text().to_owned();
        trip = pipeline.format_once(trip).expect("the trip succeeds");
        assert_ne!(trip.text(), before, "each trip counted edits");
    }
    assert_eq!(settled.text(), trip.text());
    assert!(pipeline.unsettled(&settled).is_empty());
    assert!(!trips.capped);
}
