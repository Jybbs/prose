//! Unit coverage for the harness's own arithmetic, which decides where a
//! break is blamed and which an off-by-one misattributes silently rather
//! than failing.

use std::{
    collections::BTreeMap, ops::Range, os::unix::process::ExitStatusExt, process::ExitStatus,
};

use rstest::rstest;
use ruff_source_file::LineIndex;

use crate::{
    bindings::binding_rows,
    compare::divergence,
    corpus::excluded,
    diff::{hunk, mapped_rows},
    execute::ending,
    fixes::{drops, holds_word, reaches, rewritten},
    format::{edit_rows, row_of},
    records::{EditRows, Outcome},
};

/// An outcome that ran cleanly, binding `names` and the constants `spelt`.
fn bound(names: &[&str], spelt: &[(&str, &str)]) -> Outcome {
    Outcome {
        constants: spelt
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        kind: "ok".to_owned(),
        names: names.iter().map(|name| (*name).to_owned()).collect(),
        ..Outcome::default()
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
fn a_raised_run_returns_its_error_and_name() {
    let raised = Outcome {
        error: "raises NameError: name 'x' is not defined".to_owned(),
        kind: "raised".to_owned(),
        name: Some("x".to_owned()),
        ..Outcome::default()
    };
    assert_eq!(
        divergence(&raised, &bound(&[], &[])),
        Some((raised.error.clone(), Some("x".to_owned())))
    );
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
fn a_decorated_definition_binds_on_its_def_row_rather_than_its_decorator() {
    let rows = binding_rows("@deco\n@other\ndef f():\n    pass\n\n\n@deco\nclass K:\n    pass\n");
    assert_eq!(rows.get("f"), Some(&(3..4)));
    assert_eq!(rows.get("K"), Some(&(8..9)));
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
fn binding_rows_are_empty_for_a_module_that_does_not_parse() {
    assert_eq!(binding_rows("def (\n"), BTreeMap::new());
}

#[test]
fn dropping_a_name_reads_whole_words_only() {
    let text = "from m import a, b\n";
    let edits = [edit("from m import a", 0..18, text)];
    assert!(drops(&edits, "b", text));
    assert!(!drops(&edits, "a", text));
}

#[rstest]
#[case("pkg/__main__.py")]
#[case("__main__.py")]
#[case("test/x.py")]
#[case("a/tests/b.py")]
#[case("idlelib/idle_test/x.py")]
#[case("turtledemo/x.py")]
#[case("antigravity.py")]
#[case("idlelib/idle.py")]
#[case("webbrowser.py")]
fn entry_points_leave_the_walk(#[case] relative: &str) {
    assert!(excluded(relative));
}

#[rstest]
#[case("test_x.py")]
#[case("unittest/mock.py")]
#[case("a/testing/b.py")]
#[case("re/_parser.py")]
fn library_modules_stay_in_the_walk(#[case] relative: &str) {
    assert!(!excluded(relative));
}

#[test]
fn a_signal_death_is_a_raise_rather_than_a_timeout() {
    let died = ending(ExitStatus::from_raw(11), "");
    assert_eq!(died.kind, "raised");
    assert_eq!(died.error, "ends on signal: 11 (SIGSEGV)");
}

#[test]
fn a_clean_exit_without_a_record_is_unmeasured_and_a_dirty_one_raises() {
    assert_eq!(ending(ExitStatus::from_raw(0), "").kind, "unmeasured");
    let dirty = ending(ExitStatus::from_raw(2 << 8), "boom");
    assert_eq!(dirty.kind, "raised");
    assert_eq!(dirty.error, "ends on exit status: 2, printing boom");
}

#[test]
fn identical_namespaces_do_not_diverge() {
    let same = bound(&["N"], &[("N", "1")]);
    assert_eq!(divergence(&same, &same), None);
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
fn rewritten_returns_the_reached_lines_before_and_after() {
    let text = "a = 1\nb = 2\nc = 3\n";
    assert_eq!(
        rewritten(&[edit("9", 10..11, text)], text),
        ("b = 2".to_owned(), "b = 9".to_owned())
    );
}

#[test]
fn rows_map_back_through_an_equal_a_replaced_and_an_inserted_block() {
    let before = ["x", "Y", "Q", "z"];
    let after = ["x", "y", "z"];
    assert_eq!(mapped_rows(&before, &after, 1), 1..2);
    assert_eq!(mapped_rows(&before, &after, 2), 2..4);
    assert_eq!(mapped_rows(&before, &after, 9), 0..0);
    assert_eq!(mapped_rows(&["x", "z"], &["x", "N", "z"], 2), 2..3);
}

#[test]
fn the_hunk_cuts_context_either_side_of_the_row() {
    let before: Vec<String> = (1..=10).map(|n| format!("l{n}")).collect();
    let mut after = before.clone();
    after[5] = "L6".to_owned();
    let was: Vec<&str> = before.iter().map(String::as_str).collect();
    let now: Vec<&str> = after.iter().map(String::as_str).collect();
    assert_eq!(
        hunk(&was, &now, Some(6), ""),
        [
            "...", " l4", " l5", "-l6", "+L6", " l7", " l8", " l9", "..."
        ]
    );
}

#[test]
fn the_hunk_centres_on_the_changed_line_naming_the_name() {
    let before: Vec<String> = (1..=10).map(|n| format!("l{n}")).collect();
    let mut after = before.clone();
    after[2] = "L3".to_owned();
    after[5] = "MARK".to_owned();
    let was: Vec<&str> = before.iter().map(String::as_str).collect();
    let now: Vec<&str> = after.iter().map(String::as_str).collect();
    let lines = hunk(&was, &now, None, "MARK");
    assert!(lines.iter().any(|line| line == "+MARK"));
    assert!(!lines.iter().any(|line| line == "+L3"));
}

#[test]
fn whole_word_matching_rejects_a_longer_identifier() {
    assert!(holds_word("from m import a, b", "a"));
    assert!(!holds_word("from m import ab", "a"));
    assert!(!holds_word("renamed", "name"));
}

#[test]
fn rows_count_from_one() {
    let lines = LineIndex::from_source_text("a\nbb\nccc\n");
    assert_eq!(row_of(&lines, 0), 1);
    assert_eq!(row_of(&lines, 2), 2);
    assert_eq!(row_of(&lines, 5), 3);
}
