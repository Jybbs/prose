//! Subset-surface tests over splitting a pipeline and folding part of one.

use super::*;

#[rstest]
#[case("band-constants", "group-imports", false)]
#[case("align-equals", "align-colons", true)]
fn fingerprint_reads_a_sibling_flag_off_the_selection(
    #[case] rule: &'static str,
    #[case] sibling: &'static str,
    #[case] alike: bool,
) {
    let config = Config::default();
    let rule = RuleId::from(rule);
    let alone = Pipeline::with_filters(&config, &[rule], &[]);
    let beside = Pipeline::with_filters(&config, &[rule, RuleId::from(sibling)], &[]);
    let (_, seated) = beside
        .split()
        .into_iter()
        .find(|(id, _)| *id == rule)
        .expect("the pair seats the rule");

    assert_eq!(alone.fingerprint() == seated.fingerprint(), alike);
}

#[test]
fn fingerprints_match_what_each_split_pipeline_renders() {
    let config = Config::default();
    let selected = ["align-equals", "band-constants", "group-imports"].map(RuleId::from);

    let prints = Pipeline::with_filters(&config, &selected, &[]).fingerprints();

    let split = Pipeline::with_filters(&config, &selected, &[])
        .split()
        .into_iter()
        .map(|(_, single)| single.fingerprint())
        .collect_vec();
    assert_eq!(prints, split);
}

#[test]
fn format_matches_the_text_half_of_run() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let text = "import sys\nimport os\n\n\n\n\nx  =  1\n";
    let formatted = pipeline
        .format(parse(text))
        .expect("the format run succeeds");
    let (ran, _) = pipeline.run(parse(text)).expect("the run succeeds");
    assert_eq!(formatted.text(), ran.text());
}

#[test]
fn format_short_circuits_when_file_is_suppressed() {
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let text = "# prose: off\nx = 1\n";
    let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
        id: RuleId::from("never-called"),
        log: log.clone(),
    })]);

    let formatted = pipeline.format(parse(text)).expect("short-circuit format");

    assert_eq!(formatted.text(), text);
    assert!(log.lock().expect("log mutex").is_empty());
}

#[test]
fn format_span_leaves_a_file_suppressed_source_alone() {
    let pipeline = Pipeline::from_rules(vec![Box::new(never_settles("widener"))]);
    let text = "# prose: off\nx = 1\n";

    let spanned = pipeline
        .format_span(parse(text), 0..1)
        .expect("a suppressed source folds to itself");

    assert_eq!(spanned.text(), text);
}

#[test]
fn format_span_segments_compose_to_the_full_fold() {
    // A seam falling inside a batch splits it, so the rules behind the
    // seam read a reparsed buffer rather than the one the batch opened
    // on, and the two agree only where the registry's independence
    // declarations hold.
    let pipeline = Pipeline::with_defaults(&Config::default());
    let text = concat!(
        "import os, sys\n",
        "from typing import List, Optional\n",
        "\n\n\n\n",
        "def f(xs: List[int], y: Optional[str] = None) -> Optional[int]:\n",
        "    'doc'\n",
        "    d = {\"a\": 1, \"b\": 2,}\n",
        "    msg = \"hello %s\" % y\n",
        "    s = \"one\" \"two\"\n",
        "    if y == None:\n",
        "        return None\n",
        "    match xs:\n",
        "        case [1]:\n",
        "            return 1\n",
        "        case _:\n",
        "            return 0\n",
        "y  =  2\n",
    );
    let full = pipeline
        .format(parse(text))
        .expect("the whole fold succeeds");

    for seam in 0..=pipeline.len() {
        let head = pipeline
            .format_span(parse(text), 0..seam)
            .expect("the head segment folds");
        let tail = pipeline
            .format_span(head, seam..pipeline.len())
            .expect("the tail segment folds");
        assert_eq!(
            tail.text(),
            full.text(),
            "segments split at seat {seam} differ from the whole fold",
        );
    }
}

#[test]
fn split_seats_each_rule_alone_in_pipeline_order() {
    let config = Config::default();
    let selected = ["align-equals", "band-constants", "align-colons"].map(RuleId::from);
    let singles = Pipeline::with_filters(&config, &selected, &[]).split();

    assert_eq!(
        singles.iter().map(|(id, _)| id.as_str()).collect_vec(),
        ["band-constants", "align-colons", "align-equals"],
    );
    assert!(
        singles
            .iter()
            .all(|(id, single)| registered_slugs(single) == [id.as_str()])
    );
}
