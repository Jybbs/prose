//! Unit coverage for the harness's own arithmetic, which decides where a
//! break is blamed and which an off-by-one misattributes silently rather
//! than failing.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ops::Range,
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{self, ExitStatus},
};

use itertools::Itertools;
use rstest::rstest;
use ruff_source_file::LineIndex;
use similar::TextDiff;

use crate::{
    bindings::binding_rows,
    common::SHOWN,
    compare::{compare, divergence},
    corpus::{candidates, excluded},
    diff::{hunk, mapped_rows},
    execute::{ending, module_name},
    fixes::{drops, holds_word, reaches, rewritten},
    format::{edit_rows, row_of},
    outcome::{Kind, Outcome, relative_to},
    ratchet::{Baseline, Carried, bake, dropped, judge, skipping},
    records::{Break, EditRows, Frame, Width},
    report::render,
    sweep::DEFAULT_LABEL,
};

/// An outcome that ran cleanly, binding `names` and the constants `spelt`.
fn bound(names: &[&str], spelt: &[(&str, &str)]) -> Outcome {
    Outcome {
        constants: spelt
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        kind: Kind::Ok,
        names: names
            .iter()
            .map(|name| (*name).to_owned())
            .sorted()
            .collect(),
        ..Outcome::default()
    }
}

/// A break at `frame` for `module`, diverging for `reason`.
fn broken(module: &str, frame: &str, reason: &str) -> Break {
    Break {
        attribution: String::new(),
        formatted: Outcome::default(),
        frame: Frame {
            file: frame.to_owned(),
            row: None,
        },
        hunk: Vec::new(),
        module: module.to_owned(),
        name: None,
        original: Outcome::default(),
        reason: reason.to_owned(),
    }
}

/// One edit rewriting `range` of `text` with `content`.
fn edit(content: &str, range: Range<usize>, text: &str) -> EditRows {
    EditRows {
        content: content.to_owned(),
        rows: edit_rows(&LineIndex::from_source_text(text), text, &range),
        range,
    }
}

#[test]
fn a_baked_break_set_reads_back_as_the_set_that_wrote_it() {
    let found = Width {
        breaks: vec![broken("m.py", "re/_parser.py", "leaves `X` unbound")],
        candidates: 1,
        comparable: 1,
        flaky: Vec::new(),
        label: "default".to_owned(),
        refused: 0,
        uncomparable: vec!["blocked.py".to_owned()],
        unmeasured: Vec::new(),
    };
    let baked = env::temp_dir().join(format!("prose-imports-baseline.{}", process::id()));
    bake(&baked, &[found]);
    let held: Baseline = serde_json::from_str(
        &fs_err::read_to_string(&baked).expect("the baked break set reads back"),
    )
    .expect("the baked break set parses");
    assert_eq!(
        held.breaks["default"],
        [Carried {
            file: "re/_parser.py".to_owned(),
            reason: "leaves `X` unbound".to_owned(),
        }]
        .into()
    );
    assert_eq!(
        held.uncomparable["default"],
        ["blocked.py".to_owned()].into()
    );
}

#[test]
fn a_break_reporting_no_loaded_modules_falls_back_to_its_own() {
    let mut brk = broken("m.py", "m.py", "leaves `X` unbound");
    assert_eq!(brk.loaded(), ["m.py"]);
    brk.formatted.loaded = vec!["a.py".to_owned(), "b.py".to_owned()];
    assert_eq!(brk.loaded(), ["a.py", "b.py"]);
}

#[test]
fn a_break_the_report_names_carries_its_frame_reason_and_repro() {
    let mut brk = broken(
        "_colorize.py",
        "re/_parser.py",
        "raises NameError: no MAXGROUPS",
    );
    brk.attribution = "under `prune-inert-imports`".to_owned();
    brk.frame.row = Some(111);
    brk.hunk = vec!["-from _sre import MAXGROUPS".to_owned()];
    let found = Width {
        breaks: vec![brk],
        candidates: 4,
        comparable: 3,
        flaky: Vec::new(),
        label: DEFAULT_LABEL.to_owned(),
        refused: 1,
        uncomparable: Vec::new(),
        unmeasured: Vec::new(),
    };
    let shown = render(&["kept.py".to_owned()].into(), &found);
    assert!(shown.contains("  carried          1"), "{shown}");
    assert!(shown.contains("  refused          1"), "{shown}");
    assert!(
        shown.contains(
            "re/_parser.py:111 raises NameError: no MAXGROUPS, under `prune-inert-imports`"
        ),
        "{shown}"
    );
    assert!(
        shown.contains("reproduce with mise run imports _colorize.py"),
        "{shown}"
    );
    assert!(shown.contains("-from _sre import MAXGROUPS"), "{shown}");
}

#[test]
fn a_clean_exit_without_a_record_is_unmeasured_and_a_dirty_one_raises() {
    assert_eq!(ending(ExitStatus::from_raw(0), "").kind, Kind::Unmeasured);
    let dirty = ending(ExitStatus::from_raw(2 << 8), "boom");
    assert_eq!(dirty.kind, Kind::Raised);
    assert_eq!(dirty.error, "ends on exit status: 2, printing boom");
}

#[test]
fn a_constant_rebound_names_both_values() {
    let original = bound(&["N"], &[("N", "1")]);
    let formatted = bound(&["N"], &[("N", "2")]);
    assert_eq!(
        divergence(&formatted, &original),
        Some((
            "binds `N` to 2 where the original binds 1".to_owned(),
            Some("N".to_owned())
        ))
    );
}

#[test]
fn a_constant_that_is_no_longer_plain_reads_as_missing() {
    let original = bound(&["N"], &[("N", "1")]);
    let formatted = bound(&["N"], &[]);
    let (why, _) = divergence(&formatted, &original).expect("the constant differs");
    assert_eq!(
        why,
        "binds `N` to no plain constant where the original binds 1"
    );
}

#[test]
fn a_decorated_definition_binds_on_its_def_row_rather_than_its_decorator() {
    let rows = binding_rows("@deco\n@other\ndef f():\n    pass\n\n\n@deco\nclass K:\n    pass\n");
    assert_eq!(rows.get("f"), Some(&(3..4)));
    assert_eq!(rows.get("K"), Some(&(8..9)));
}

#[test]
fn a_dropped_name_and_an_added_name_report_their_direction() {
    let original = bound(&["a", "b"], &[]);
    let formatted = bound(&["a"], &[]);
    assert_eq!(
        divergence(&formatted, &original),
        Some(("leaves `b` unbound".to_owned(), Some("b".to_owned())))
    );
    assert_eq!(
        divergence(&original, &formatted),
        Some((
            "binds `b` the original does not".to_owned(),
            Some("b".to_owned())
        ))
    );
}

#[rstest]
#[case("os.py", "os")]
#[case("asyncio/queues.py", "asyncio.queues")]
#[case("asyncio/__init__.py", "asyncio")]
#[case("importlib/metadata/__init__.py", "importlib.metadata")]
fn a_module_path_binds_the_name_an_import_binds(#[case] module: &str, #[case] dotted: &str) {
    assert_eq!(module_name(module), dotted);
}

#[test]
fn a_path_names_itself_against_the_first_tree_carrying_it() {
    let trees = [Path::new("/formatted"), Path::new("/original")];
    assert_eq!(
        relative_to("/formatted/m.py", &trees),
        Some("m.py".to_owned())
    );
    assert_eq!(
        relative_to("/original/m.py", &trees),
        Some("m.py".to_owned())
    );
    assert_eq!(relative_to("/elsewhere/m.py", &trees), None);
}

#[test]
fn a_raise_row_composes_its_sentence_beside_the_other_endings() {
    let record = [
        ["kind", "raised"].join("\0"),
        ["raise", "NameError", "name 'x' is not defined"].join("\0"),
        ["missing", "x"].join("\0"),
    ]
    .join("\u{1e}");
    let read = Outcome::parse(&record, &[Path::new("/tree")]);
    assert_eq!(read.kind, Kind::Raised);
    assert_eq!(read.error, "raises NameError: name 'x' is not defined");
    assert_eq!(read.name, Some("x".to_owned()));
}

#[test]
fn a_raised_run_returns_its_error_and_name() {
    let raised = Outcome {
        error: "raises NameError: name 'x' is not defined".to_owned(),
        kind: Kind::Raised,
        name: Some("x".to_owned()),
        ..Outcome::default()
    };
    assert_eq!(
        divergence(&raised, &bound(&[], &[])),
        Some((raised.error.clone(), Some("x".to_owned())))
    );
}

#[test]
fn a_signal_death_is_a_raise_rather_than_a_timeout() {
    let died = ending(ExitStatus::from_raw(11), "");
    assert_eq!(died.kind, Kind::Raised);
    assert_eq!(died.error, "ends on signal: 11 (SIGSEGV)");
}

#[test]
fn a_timing_out_break_counts_as_a_module_rather_than_a_defect() {
    let timed = |module: &str| {
        let mut brk = broken(module, "socket.py", "times out after 30s");
        brk.formatted = Outcome::of(Kind::Timeout, "times out after 30s");
        brk
    };
    let found = Width {
        breaks: vec![timed("a.py"), timed("b.py")],
        candidates: 2,
        comparable: 2,
        flaky: Vec::new(),
        label: DEFAULT_LABEL.to_owned(),
        refused: 0,
        uncomparable: Vec::new(),
        unmeasured: Vec::new(),
    };
    assert_eq!(found.timing_out(), 2);
    let shown = render(&BTreeSet::new(), &found);
    assert!(shown.contains("  timeouts         2"), "{shown}");
    assert!(shown.contains("times out (1):"), "{shown}");
}

#[test]
fn a_walrus_and_a_type_alias_bind_at_module_level() {
    let rows = binding_rows(
        "if (n := go()):\n    pass\n\nwhile (m := go()):\n    pass\n\ntype Alias = int\n",
    );
    assert_eq!(rows.get("n"), Some(&(1..3)));
    assert_eq!(rows.get("m"), Some(&(4..6)));
    assert_eq!(rows.get("Alias"), Some(&(7..8)));
}

#[test]
fn a_walrus_in_a_match_subject_binds_at_module_level() {
    let rows = binding_rows("match (n := go()):\n    case _:\n        pass\n");
    assert_eq!(rows.get("n"), Some(&(1..4)));
}

#[test]
fn an_end_at_column_one_closes_on_the_row_above() {
    let text = "a = 1\nb = 2\nc = 3\n";
    let lines = LineIndex::from_source_text(text);
    assert_eq!(edit_rows(&lines, text, &(0..12)), 1..3);
    assert_eq!(edit_rows(&lines, text, &(0..13)), 1..4);
    assert_eq!(edit_rows(&lines, text, &(0..5)), 1..2);
}

#[test]
fn an_import_binds_its_first_dotted_segment() {
    let rows =
        binding_rows("import os.path\nimport xml.etree.ElementTree as et\nfrom a.b import c\n");
    assert_eq!(rows.get("os"), Some(&(1..2)));
    assert_eq!(rows.get("et"), Some(&(2..3)));
    assert_eq!(rows.get("c"), Some(&(3..4)));
    assert!(!rows.contains_key("xml"));
}

#[test]
fn an_unrecognised_kind_row_reads_as_unmeasured() {
    let read = Outcome::parse(&["kind", "wat"].join("\0"), &[]);
    assert_eq!(read.kind, Kind::Unmeasured);
}

#[test]
fn binding_rows_are_empty_for_a_module_that_does_not_parse() {
    assert_eq!(binding_rows("def (\n"), BTreeMap::new());
}

#[test]
fn binding_rows_reach_tuple_and_starred_targets() {
    let rows = binding_rows("STRICT, CONFORM = boundary()\nhead, *rest = xs\n[a, b] = pair\n");
    for name in ["STRICT", "CONFORM", "head", "rest", "a", "b"] {
        assert!(rows.contains_key(name), "{name} binds at module level");
    }
}

#[test]
fn binding_rows_walk_compound_statements_and_skip_nested_scopes() {
    let rows = binding_rows(
        "import os.path as osp\nfrom re import compile as rc\n\n\ndef f(a):\n    inner = 1\n\n\nclass K:\n    attr = 2\n\n\ntry:\n    t = 1\nexcept ValueError:\n    e = 2\n\nfor i in y:\n    pass\n",
    );
    assert_eq!(rows.get("osp"), Some(&(1..2)));
    assert_eq!(rows.get("rc"), Some(&(2..3)));
    assert_eq!(rows.get("f"), Some(&(5..6)));
    assert_eq!(rows.get("K"), Some(&(9..10)));
    assert_eq!(rows.get("t"), Some(&(14..15)));
    assert_eq!(rows.get("e"), Some(&(16..17)));
    assert_eq!(rows.get("i"), Some(&(18..20)));
    for absent in ["inner", "attr", "a"] {
        assert!(
            !rows.contains_key(absent),
            "{absent} binds in a nested scope"
        );
    }
}

#[test]
fn candidates_drop_the_entry_points_from_the_rewritten_set() {
    let rewritten = ["os.py", "test/x.py", "turtledemo/y.py", "re/_parser.py"]
        .map(str::to_owned)
        .into();
    assert_eq!(candidates(&rewritten), ["os.py", "re/_parser.py"]);
}

#[test]
fn comparing_sorts_each_module_into_broken_comparable_or_unmeasured() {
    let modules = [
        "gone.py".to_owned(),
        "kept.py".to_owned(),
        "lost.py".to_owned(),
    ];
    let before = [
        ("gone.py".to_owned(), bound(&["a", "b"], &[])),
        ("kept.py".to_owned(), bound(&["a"], &[])),
        ("lost.py".to_owned(), bound(&["a"], &[])),
    ]
    .into();
    let after = [
        ("gone.py".to_owned(), bound(&["a"], &[])),
        ("kept.py".to_owned(), bound(&["a"], &[])),
        (
            "lost.py".to_owned(),
            Outcome::of(Kind::Unmeasured, "left no record"),
        ),
    ]
    .into();
    let found = compare(&after, &before, &modules);
    assert_eq!(found.comparable, 2);
    assert_eq!(found.unmeasured, ["lost.py".to_owned()]);
    assert_eq!(found.uncomparable, Vec::<String>::new());
    assert_eq!(found.breaks.len(), 1);
    assert_eq!(found.breaks[0].module, "gone.py");
    assert_eq!(found.breaks[0].reason, "leaves `b` unbound");
}

#[test]
fn dropping_a_name_reads_whole_words_only() {
    let text = "from m import a, b\n";
    let edits = [edit("from m import a", 0..18, text)];
    assert!(drops(&edits, "b", text));
    assert!(!drops(&edits, "a", text));
}

#[rstest]
fn entry_points_leave_the_walk(
    #[values(
        "pkg/__main__.py",
        "__main__.py",
        "test/x.py",
        "a/tests/b.py",
        "idlelib/idle_test/x.py",
        "turtledemo/x.py",
        "antigravity.py",
        "idlelib/idle.py",
        "webbrowser.py"
    )]
    relative: &str,
) {
    assert!(excluded(relative));
}

#[test]
fn identical_namespaces_do_not_diverge() {
    let same = bound(&["N"], &[("N", "1")]);
    assert_eq!(divergence(&same, &same), None);
}

#[rstest]
fn library_modules_stay_in_the_walk(
    #[values("test_x.py", "unittest/mock.py", "a/testing/b.py", "re/_parser.py")] relative: &str,
) {
    assert!(!excluded(relative));
}

#[test]
fn parsing_a_record_filters_loader_names_and_reads_frames() {
    let record = [
        ["kind", "ok"].join("\0"),
        ["bound", "__file__"].join("\0"),
        ["bound", "__all__"].join("\0"),
        ["bound", "N"].join("\0"),
        ["const", "__all__", "('a',)"].join("\0"),
        ["const", "N", "1"].join("\0"),
        ["frame", "9", "/tree/m.py"].join("\0"),
        ["loaded", "/tree/m.py"].join("\0"),
        ["loaded", "/elsewhere/other.py"].join("\0"),
    ]
    .join("\u{1e}");
    let read = Outcome::parse(&record, &[Path::new("/tree")]);
    assert_eq!(read.kind, Kind::Ok);
    assert_eq!(read.names, ["N", "__all__"]);
    assert_eq!(read.constants, [("N".to_owned(), "1".to_owned())].into());
    assert_eq!(read.frames, [("/tree/m.py".to_owned(), 9)]);
    assert_eq!(read.loaded, ["m.py"]);
}

#[test]
fn reaching_reads_a_row_overlap_or_a_written_line() {
    let text = "a = 1\nb = 2\nc = 3\nd = 4\ne = 5\n";
    let edits = [edit("x = 1\n", 12..18, text)];
    assert!(reaches(&edits, &(3..5), ""));
    assert!(!reaches(&edits, &(5..6), ""));
    assert!(reaches(&edits, &(9..10), "x = 1"));
    assert!(!reaches(&edits, &(9..10), "x = 2"));
}

#[test]
fn rewritten_returns_nothing_for_no_edits_or_a_span_past_the_text() {
    let text = "a = 1\n";
    assert_eq!(rewritten(&[], text), (String::new(), String::new()));
    let beyond = EditRows {
        content: "x".to_owned(),
        range: 0..99,
        rows: 1..2,
    };
    assert_eq!(rewritten(&[beyond], text), (String::new(), String::new()));
}

#[test]
fn rewritten_returns_the_reached_lines_before_and_after() {
    let text = "a = 1\nb = 2\nc = 3\n";
    assert_eq!(
        rewritten(&[edit("9", 10..11, text)], text),
        ("b = 2".to_owned(), "b = 9".to_owned())
    );
}

#[test]
fn rows_count_from_one() {
    let lines = LineIndex::from_source_text("a\nbb\nccc\n");
    assert_eq!(row_of(&lines, 0), 1);
    assert_eq!(row_of(&lines, 2), 2);
    assert_eq!(row_of(&lines, 5), 3);
}

#[test]
fn rows_map_back_through_an_equal_a_replaced_and_an_inserted_block() {
    let before = ["x", "Y", "Q", "z"];
    let after = ["x", "y", "z"];
    let diff = TextDiff::from_slices(&before, &after);
    assert_eq!(mapped_rows(&diff, 1), 1..2);
    assert_eq!(mapped_rows(&diff, 2), 2..4);
    assert_eq!(mapped_rows(&diff, 9), 0..0);
    let inserted = TextDiff::from_slices(&["x", "z"], &["x", "N", "z"]);
    assert_eq!(mapped_rows(&inserted, 2), 2..3);
}

#[test]
fn the_flaky_list_caps_at_the_shown_limit() {
    let found = Width {
        breaks: Vec::new(),
        candidates: SHOWN + 3,
        comparable: SHOWN + 3,
        flaky: (0..SHOWN + 3).map(|n| format!("m{n}.py")).collect(),
        label: DEFAULT_LABEL.to_owned(),
        refused: 0,
        uncomparable: Vec::new(),
        unmeasured: Vec::new(),
    };
    let shown = render(&BTreeSet::new(), &found);
    assert!(
        shown.contains(&format!(
            "flaky, a second run did not confirm it ({}):",
            SHOWN + 3
        )),
        "{shown}"
    );
    assert!(shown.contains("... and 3 more"), "{shown}");
    assert!(!shown.contains("m32.py"), "{shown}");
}

#[test]
fn the_hunk_centres_on_the_changed_line_naming_the_name() {
    let before: Vec<String> = (1..=10).map(|n| format!("l{n}")).collect();
    let mut after = before.clone();
    after[2] = "L3".to_owned();
    after[5] = "MARK".to_owned();
    let was: Vec<&str> = before.iter().map(String::as_str).collect();
    let now: Vec<&str> = after.iter().map(String::as_str).collect();
    let lines = hunk(&TextDiff::from_slices(&was, &now), None, "MARK");
    assert!(lines.iter().any(|line| line == "+MARK"));
    assert!(!lines.iter().any(|line| line == "+L3"));
}

#[test]
fn the_hunk_cuts_context_either_side_of_the_row() {
    let before: Vec<String> = (1..=10).map(|n| format!("l{n}")).collect();
    let mut after = before.clone();
    after[5] = "L6".to_owned();
    let was: Vec<&str> = before.iter().map(String::as_str).collect();
    let now: Vec<&str> = after.iter().map(String::as_str).collect();
    assert_eq!(
        hunk(&TextDiff::from_slices(&was, &now), Some(6), ""),
        [
            "...", " l4", " l5", "-l6", "+L6", " l7", " l8", " l9", "..."
        ]
    );
}

#[test]
fn the_hunk_falls_back_to_the_first_change_with_no_row_or_name() {
    let before: Vec<String> = (1..=10).map(|n| format!("l{n}")).collect();
    let mut after = before.clone();
    after[6] = "L7".to_owned();
    let was: Vec<&str> = before.iter().map(String::as_str).collect();
    let now: Vec<&str> = after.iter().map(String::as_str).collect();
    let lines = hunk(&TextDiff::from_slices(&was, &now), None, "");
    assert!(lines.iter().any(|line| line == "-l7"), "{lines:?}");
    assert!(lines.iter().any(|line| line == "+L7"), "{lines:?}");
}

#[test]
fn the_ratchet_carries_a_break_the_baseline_holds_at_the_same_width() {
    let found = Width {
        breaks: vec![broken("m.py", "re/_parser.py", "leaves `X` unbound")],
        candidates: 10,
        comparable: 7,
        flaky: Vec::new(),
        label: "default".to_owned(),
        refused: 0,
        uncomparable: vec!["a.py".to_owned(), "b.py".to_owned()],
        unmeasured: vec!["u.py".to_owned()],
    };
    let held = Baseline {
        breaks: [(
            "default".to_owned(),
            [Carried {
                file: "re/_parser.py".to_owned(),
                reason: "leaves `X` unbound".to_owned(),
            }]
            .into(),
        )]
        .into(),
        uncomparable: [("default".to_owned(), ["a.py".to_owned()].into())].into(),
    };
    assert_eq!(judge(&found, &held), ["m.py".to_owned()].into());
    assert_eq!(judge(&found, &Baseline::default()), BTreeSet::new());
    assert_eq!(found.uncomparable.len(), 2);
    assert_eq!(dropped(&found, &held), ["b.py".to_owned()].into());
    assert_eq!(
        skipping(&held, "default"),
        Some(&["a.py".to_owned()].into())
    );
}

#[test]
fn the_summary_block_holds_every_count_in_one_column() {
    let found = Width {
        breaks: Vec::new(),
        candidates: 12,
        comparable: 9,
        flaky: Vec::new(),
        label: DEFAULT_LABEL.to_owned(),
        refused: 0,
        uncomparable: vec!["a.py".to_owned(), "b.py".to_owned(), "c.py".to_owned()],
        unmeasured: Vec::new(),
    };
    assert_eq!(
        render(&BTreeSet::new(), &found),
        concat!(
            "  candidates      12\n",
            "  comparable       9\n",
            "  uncomparable     3\n",
            "  breaks           0\n",
            "  timeouts         0\n",
            "  flaky            0",
        )
    );
}

#[test]
fn whole_word_matching_rejects_a_longer_identifier() {
    assert!(holds_word("from m import a, b", "a"));
    assert!(!holds_word("from m import ab", "a"));
    assert!(!holds_word("renamed", "name"));
}
