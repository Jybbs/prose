//! Tests over `Pipeline::run`.

use super::*;

#[test]
fn compile_failure_surfaces_rule_id() {
    let pipeline = Pipeline::from_rules(vec![Box::new(breaks_compile())]);
    let source = parse(FUTURE_LEAD);

    let err = pipeline.run(source).expect_err("compile check should fail");

    assert_matches!(err, PipelineError::Compile { rule, .. } if rule.as_str() == "breaks-compile");
}

#[test]
fn downstream_rule_apply_sees_upstream_rewritten_text() {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let pipeline = Pipeline::from_rules(vec![
        Box::new(TextCapturingRule {
            edits: vec![Edit::range_replacement("y".to_owned(), range(0, 1))],
            id: RuleId::from("rewrite-x-to-y"),
            seen: seen.clone(),
        }),
        Box::new(TextCapturingRule {
            edits: Vec::new(),
            id: RuleId::from("downstream-observer"),
            seen: seen.clone(),
        }),
    ]);
    let source = parse("x = 1\n");

    pipeline.run(source).expect("both stages succeed");

    assert_eq!(*seen.lock().expect("seen mutex"), ["x = 1\n", "y = 1\n"]);
}

#[test]
fn empty_pipeline_returns_identical_source() {
    let pipeline = Pipeline::from_rules(Vec::new());
    let source = parse("x = 1\n");

    let (result, diagnostics) = pipeline.run(source).expect("identity run succeeds");

    assert_eq!(result.text(), "x = 1\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn reparse_failure_surfaces_rule_id() {
    let pipeline = Pipeline::from_rules(vec![Box::new(breaks_parse())]);
    let source = parse("x = 1\n");

    assert_matches!(
        pipeline.run(source),
        Err(PipelineError::Reparse { rule, .. }) if rule.as_str() == "breaks-parse"
    );
}

#[test]
fn rules_run_in_registration_order() {
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let pipeline = Pipeline::from_rules(vec![
        Box::new(SentinelRule {
            id: RuleId::from("first"),
            log: log.clone(),
        }),
        Box::new(SentinelRule {
            id: RuleId::from("second"),
            log: log.clone(),
        }),
        Box::new(SentinelRule {
            id: RuleId::from("third"),
            log: log.clone(),
        }),
    ]);
    let source = parse("x = 1\n");

    pipeline.run(source).expect("all rules succeed");

    assert_eq!(
        *log.lock().expect("log mutex"),
        ["first", "second", "third"]
    );
}

#[test]
fn run_applies_a_reordering_rule_on_a_notebook() {
    // A sibling reorder runs cell-aware on a notebook, so its
    // rewrite lands inside the cell that holds the members.
    let pipeline = Pipeline::from_rules(vec![Box::new(rewrites_x_to_y())]);
    let source = notebook(&["x = 1"]);

    let (result, diagnostics) = pipeline.run(source).expect("notebook run succeeds");

    assert_eq!(result.text(), "y = 1\n");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn run_declines_an_overlapping_group_as_a_no_op() {
    let pipeline = Pipeline::from_rules(vec![Box::new(self_overlapping())]);
    let source = parse("x = 1\n");

    let (result, diagnostics) = pipeline
        .run(source)
        .expect("overlap degrades, run continues");

    assert_eq!(result.text(), "x = 1\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn run_drops_a_whole_group_holding_one_suppressed_edit() {
    // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
    //         |0--------|11----|17--------|27----|33
    // The group bundles an edit at 11..16 (inside the suppressed
    // [0..17) span) with one at 27..32. The group drops as a unit,
    // so the unsuppressed edit never applies alone.
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![
            Edit::range_replacement("y".to_owned(), range(11, 16)),
            Edit::range_replacement("Z".to_owned(), range(27, 32)),
        ]],
        id: RuleId::from("rewrite-x-and-z"),
    })]);
    let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

    let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

    assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nz = 9\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn run_drops_edits_whose_range_overlaps_a_suppressed_span() {
    // Source: "# fmt: off\nx = 1\n# fmt: on\nz = 9\n"
    //         |0--------|11----|17--------|27----|33
    // Edit at 11..16 (`x = 1`) sits inside the suppressed
    // [0..17) span and must be dropped, leaving the unsuppressed
    // edit at 27..32 (`z = 9`) in its own group to apply.
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: singleton_groups(vec![
            Edit::range_replacement("y".to_owned(), range(11, 16)),
            Edit::range_replacement("Z".to_owned(), range(27, 32)),
        ]),
        id: RuleId::from("rewrite-x-and-z"),
    })]);
    let source = parse("# fmt: off\nx = 1\n# fmt: on\nz = 9\n");

    let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

    assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\nZ\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-and-z");
}

#[test]
fn run_emits_lint_diagnostic_without_fix_per_lint_range() {
    let pipeline = Pipeline::from_rules(vec![Box::new(LintSentinelRule {
        id: RuleId::from("flag-stuff"),
        ranges: vec![range(0, 5), range(6, 11)],
    })]);
    let source = parse("x = 1\ny = 2\n");

    let (result, diagnostics) = pipeline.run(source).expect("lint-only run succeeds");

    assert_eq!(result.text(), "x = 1\ny = 2\n");
    assert_eq!(diagnostics.len(), 2);
    for diagnostic in &diagnostics {
        assert_eq!(diagnostic.severity, Severity::Lint);
        assert!(diagnostic.fix.is_none());
        assert_eq!(diagnostic.rule.as_str(), "flag-stuff");
        assert_eq!(diagnostic.message, "lint test rule");
    }
}

#[test]
fn run_emits_one_diagnostic_per_group_carrying_every_edit() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![
            Edit::range_replacement("Y".to_owned(), range(0, 1)),
            Edit::range_replacement("Z".to_owned(), range(4, 5)),
        ]],
        id: RuleId::from("rewrite-x-and-1"),
    })]);
    let source = parse("x = 1\n");

    let (result, diagnostics) = pipeline.run(source).expect("grouped rewrite succeeds");

    assert_eq!(result.text(), "Y = Z\n");
    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0]
        .fix
        .as_ref()
        .expect("format diagnostic carries a fix");
    assert_eq!(fix.edits().len(), 2);
    assert_eq!(diagnostics[0].range, range(0, 5));
}

#[test]
fn run_emits_one_diagnostic_per_surviving_edit() {
    let pipeline = Pipeline::from_rules(vec![Box::new(rewrites_x_to_y())]);
    let source = parse("x = 1\n");

    let (result, diagnostics) = pipeline.run(source).expect("rewrite succeeds");

    assert_eq!(result.text(), "y = 1\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule.as_str(), "rewrite-x-to-y");
    assert_eq!(diagnostics[0].severity, Severity::Format);
    assert!(diagnostics[0].fix.is_some());
}

#[test]
#[should_panic(expected = "emitted a duplicate edit")]
fn run_flags_a_byte_identical_duplicate_edit() {
    let edit = Edit::range_replacement("y".to_owned(), range(0, 1));
    let rule = GroupSentinelRule {
        groups: vec![vec![edit.clone()], vec![edit]],
        id: RuleId::from("duplicating"),
    };
    let pipeline = Pipeline::from_rules(vec![Box::new(rule)]);
    let _ = pipeline.run(parse("x = 1\n"));
}

#[test]
fn run_resolves_a_lint_range_against_the_settled_source() {
    // The lint rule registers ahead of the rewriting rule, which
    // inserts a line above the ignored statement. Collecting lints
    // after the rewrites settle keeps the lint's range on the row
    // carrying the directive, so the ignore still matches.
    let pipeline = Pipeline::from_rules(vec![
        Box::new(NeedleLintRule {
            id: RuleId::from("single-use-variables"),
            needle: "y = 2",
        }),
        Box::new(GroupSentinelRule {
            groups: vec![vec![Edit::insertion(
                "a = 0\n".to_owned(),
                TextSize::new(0),
            )]],
            id: RuleId::from("prepend-a"),
        }),
    ]);
    let source = parse("x = 1\ny = 2  # prose: ignore[single-use-variables]\n");

    let (result, diagnostics) = pipeline.run(source).expect("prepend run succeeds");

    assert_eq!(
        result.text(),
        "a = 0\nx = 1\ny = 2  # prose: ignore[single-use-variables]\n",
    );
    assert!(diagnostics.iter().all(|d| !d.severity.is_lint()));
}

#[test]
fn run_short_circuits_when_file_is_suppressed() {
    let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let pipeline = Pipeline::from_rules(vec![Box::new(SentinelRule {
        id: RuleId::from("never-called"),
        log: log.clone(),
    })]);
    let source = parse("# prose: off\nx = 1\n");

    let (result, diagnostics) = pipeline.run(source).expect("short-circuit run");

    assert_eq!(result.text(), "# prose: off\nx = 1\n");
    assert!(diagnostics.is_empty());
    assert!(log.lock().expect("log mutex").is_empty());
}

#[test]
fn run_skips_empty_group_without_emitting_a_diagnostic() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![Vec::new()],
        id: RuleId::from("emits-empty-group"),
    })]);
    let source = parse("x = 1\n");

    let (result, diagnostics) = pipeline.run(source).expect("empty-group run succeeds");

    assert_eq!(result.text(), "x = 1\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn run_skips_reparse_when_every_edit_is_suppressed() {
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement("y".to_owned(), range(11, 16))]],
        id: RuleId::from("rewrite-x-to-y"),
    })]);
    let source = parse("# fmt: off\nx = 1\n# fmt: on\n");

    let (result, diagnostics) = pipeline.run(source).expect("filtered run succeeds");

    assert_eq!(result.text(), "# fmt: off\nx = 1\n# fmt: on\n");
    assert!(diagnostics.is_empty());
}

#[test]
fn run_skips_the_compile_gate_when_the_input_does_not_compile() {
    // The demoted `__future__` import arrives in the source, so the
    // rewrite of `os` to `sys` leaves the module exactly as
    // uncompilable as it was found and the run carries it through.
    let pipeline = Pipeline::from_rules(vec![Box::new(GroupSentinelRule {
        groups: vec![vec![Edit::range_replacement("sys".to_owned(), range(7, 9))]],
        id: RuleId::from("rewrite-os-to-sys"),
    })]);
    let source = parse("import os\nfrom __future__ import annotations\n");

    let (result, _) = pipeline
        .run(source)
        .expect("disarmed gate lets the run pass");

    assert_eq!(
        result.text(),
        "import sys\nfrom __future__ import annotations\n"
    );
}
