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
fn format_matches_the_text_half_of_run() {
    let pipeline = Pipeline::with_defaults(&Config::default());
    let text = "import sys\nimport os\n\n\n\n\nx  =  1\n";
    let formatted = pipeline.format(text.parse().unwrap()).unwrap();
    let (ran, _) = pipeline.run(text.parse().unwrap()).unwrap();
    assert_eq!(formatted.text(), ran.text());
}

#[test]
fn format_short_circuits_when_file_is_suppressed() {
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
        id: RuleId::from("never-called"),
        log: log.clone(),
    })]);

    let formatted = pipeline
        .format(parse("# prose: off\nx = 1\n"))
        .expect("short-circuit format");

    assert_eq!(formatted.text(), "# prose: off\nx = 1\n");
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
    let pipeline = Pipeline::with_defaults(&Config::default());
    let text = "import sys\nimport os\n\n\n\n\ny  =  2\n";
    let source: Source = text.parse().unwrap();
    let copy = source.clone();
    let full = pipeline.format(source).unwrap();
    let seam = pipeline.len() / 2;
    let head = pipeline.format_span(copy, 0..seam).unwrap();
    let tail = pipeline.format_span(head, seam..pipeline.len()).unwrap();
    assert_eq!(tail.text(), full.text());
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
