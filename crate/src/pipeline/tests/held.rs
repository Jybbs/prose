//! Held-surface tests: what the lint pass records about the run's
//! directives, the help a skip adds to a finding, and the skip
//! comparison a rule's audit reads.

use super::*;

/// Test-only rule whose audit reports one lint per finding the run's
/// ignores silenced, its message naming the line and the rule.
#[derive(Debug)]
struct SilencedEcho;

impl Rule for SilencedEcho {
    fn audit(&self, held: &Held<'_>) -> Vec<Diagnostic> {
        held.silenced()
            .iter()
            .map(|(line, rule)| {
                Diagnostic::lint(self.id(), TextRange::default(), format!("{line}:{rule}"))
            })
            .collect()
    }

    fn id(&self) -> RuleId {
        RuleId::from("echo-silenced")
    }
}

/// One lint sentinel under `flag-x` reporting the `x` it finds.
fn flag_x() -> Vec<Box<dyn Rule>> {
    vec![Box::new(NeedleLintRule {
        id: RuleId::from("flag-x"),
        needle: "x",
    })]
}

/// The `Held` record over `source` for `rules`, silencing nothing.
fn held<'a>(source: &'a Source, rules: &'a [Box<dyn Rule>]) -> Held<'a> {
    Held::new(source, rules, Vec::new())
}

#[rstest]
#[case::bare_skip("x = 1  # prose: skip\n", true)]
#[case::skip_closing_a_wrapped_statement("x = (\n    1\n)  # fmt: skip\n", true)]
#[case::no_skip("x = 1\n", false)]
#[case::ignore_naming_another_rule("x = 1  # prose: skip  # prose: ignore[align-equals]\n", false)]
#[case::skip_on_another_line("x = 1\ny = 2  # prose: skip\n", false)]
fn a_finding_closes_on_its_ignore_only_under_a_skip_with_no_ignore(
    #[case] src: &str,
    #[case] helped: bool,
) {
    let source = parse(src);
    let expected = if helped {
        "needle lint test rule. A `skip` holds out rewrites alone, so holding this finding \
         out takes `# prose: ignore[flag-x]` on its line"
    } else {
        "needle lint test rule"
    };

    let lints = settled_lints(&flag_x(), &source);

    assert_matches!(lints.as_slice(), [lint] if lint.message == expected);
}

#[rstest]
#[case::comment_beside_a_neighbors_note("connect(host)  # fmt: skip\nvalue = 1  # note\n", false)]
#[case::note_kept_beside_the_skip("value = 1  # note  # fmt: skip\nlong_name = 2  # other\n", true)]
fn a_skip_is_judged_with_its_comment_gone(#[case] src: &str, #[case] holds: bool) {
    let source = parse(src);
    let rules: Vec<Box<dyn Rule>> = vec![Box::new(AlignComments::from_config(&Config::default()))];

    assert_eq!(held(&source, &rules).skip_holds(0), holds);
}

#[test]
fn selects_reads_the_rules_the_run_carries() {
    let source = parse("x = 1\n");
    let rules = flag_x();
    let held = held(&source, &rules);

    assert!(held.selects(RuleId::from("flag-x")));
    assert!(!held.selects(AlignEquals::SLUG));
}

#[test]
fn settled_lints_hands_each_silenced_finding_to_an_audit_the_ignore_cannot_reach() {
    let source = parse("x = 1  # prose: ignore\n");
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(NeedleLintRule {
            id: SignatureAnnotations::SLUG,
            needle: "x",
        }),
        Box::new(SilencedEcho),
    ];

    let lints = settled_lints(&rules, &source);

    assert_matches!(
        lints.as_slice(),
        [echo] if echo.rule == RuleId::from("echo-silenced")
            && echo.message == "1:signature-annotations"
    );
}

#[rstest]
#[case::row_an_aligner_holds_out("host_name = 1\nport = 2  # prose: skip\n", true)]
#[case::row_holding_its_neighbors_column(
    "host = 1\ntimeout_seconds = 2  # prose: skip[align-equals]\n",
    true
)]
#[case::line_no_rule_rewrites("connect(host)  # prose: skip\n", false)]
#[case::rule_the_skip_does_not_name(
    "host = 1\ntimeout_seconds = 2  # prose: skip[strip-trailing-commas]\n",
    false
)]
fn skip_holds_where_removing_it_changes_a_named_rules_edits(
    #[case] src: &str,
    #[case] holds: bool,
) {
    let source = parse(src);
    let rules: Vec<Box<dyn Rule>> = vec![Box::new(AlignEquals::from_config(&Config::default()))];

    assert_eq!(held(&source, &rules).skip_holds(0), holds);
}
