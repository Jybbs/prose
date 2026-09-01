//! Unit coverage for the shared corpus-sweep harness, the unified-diff
//! excerpts a report shows beside a defect and the tally that keys those
//! defects by wording.

use std::path::Path;

use rstest::rstest;

use common::{EXCERPT, Hit, SHOWN, Tally, excerpt, unread};

mod common;

/// `count` lines reading `word 1` through `word count`.
fn numbered(word: &str, count: usize) -> String {
    (1..=count).map(|n| format!("{word} {n}\n")).collect()
}

#[test]
fn excerpt_counts_both_the_lines_and_the_hunks_past_its_cap() {
    let before = numbered("line", 60);
    let after: String = (1..=60)
        .map(|n| {
            if n <= 30 || n == 55 {
                format!("row {n}\n")
            } else {
                format!("line {n}\n")
            }
        })
        .collect();

    let shown = excerpt("before", "after", &before, &after);

    assert!(shown.ends_with(" more lines and 1 more hunks"), "{shown}");
}

#[test]
fn excerpt_counts_the_lines_past_its_cap() {
    let before = numbered("line", 40);
    let after = numbered("row", 40);

    let shown = excerpt("before", "after", &before, &after);

    assert_eq!(shown.lines().count(), 2 + EXCERPT + 1, "{shown}");
    assert!(shown.ends_with(" more lines"), "{shown}");
}

#[test]
fn excerpt_ends_on_the_hunk_when_nothing_is_cut() {
    let before = numbered("line", 10);
    let after = before.replace("line 2\n", "line two\n");

    let shown = excerpt("before", "after", &before, &after);

    assert!(!shown.contains("..."), "{shown}");
    assert_eq!(shown.lines().count(), 2 + 7, "{shown}");
}

#[test]
fn excerpt_is_empty_when_the_texts_match() {
    assert!(excerpt("before", "after", "x = 1\n", "x = 1\n").is_empty());
}

#[test]
fn excerpt_shows_the_first_hunk_and_counts_the_rest() {
    let before = numbered("line", 60);
    let after = before
        .replace("line 2\n", "line two\n")
        .replace("line 50\n", "line fifty\n");

    let shown = excerpt("first pass", "second pass", &before, &after);

    assert!(
        shown.starts_with("--- first pass\n+++ second pass\n@@"),
        "{shown}"
    );
    assert!(shown.contains("-line 2\n+line two\n"), "{shown}");
    assert!(shown.ends_with("... and 1 more hunks"), "{shown}");
}

#[test]
fn tally_names_a_defect_once_across_clauses_and_keeps_the_earliest_example() {
    let hit = |label: &str, width| Hit {
        clause: Some((label.to_owned(), width)),
        ..Hit::default()
    };
    let mut tally = Tally::default();
    tally.record_hit(
        "still editing".to_owned(),
        Path::new("b.py"),
        hit("code", 88),
    );
    tally.record_hit(
        "still editing".to_owned(),
        Path::new("a.py"),
        hit("import", 60),
    );
    tally.record_hit(
        "still editing".to_owned(),
        Path::new("a.py"),
        hit("code", 40),
    );

    let rendered = tally.render("defects");

    assert_eq!(tally.len(), 1);
    assert!(
        rendered.contains("still editing (2 files, e.g. a.py at code 40)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("reached at code 40, 88 and import 60"),
        "{rendered}"
    );
}

#[test]
fn tally_render_caps_the_defects_it_prints_and_counts_the_rest() {
    let mut tally = Tally::default();
    for n in 0..=SHOWN {
        tally.record_hit(format!("defect {n:02}"), Path::new("a.py"), Hit::default());
    }

    let rendered = tally.render("defects");

    assert!(
        rendered.contains(&format!("defect {:02}", SHOWN - 1)),
        "{rendered}"
    );
    assert!(
        !rendered.contains(&format!("defect {SHOWN:02}")),
        "{rendered}"
    );
    assert!(rendered.ends_with("... and 1 more"), "{rendered}");
}

#[test]
fn tally_render_carries_the_example_repro_and_detail() {
    let mut tally = Tally::default();
    let hit = Hit {
        detail: Some("--- a\n+++ b".to_owned()),
        repro: Some("cargo test".to_owned()),
        ..Hit::default()
    };
    tally.record_hit("still editing".to_owned(), Path::new("a.py"), hit);

    let rendered = tally.render("defects");

    assert!(
        rendered.contains("still editing (1 file, e.g. a.py)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("\n    reproduce with cargo test"),
        "{rendered}"
    );
    assert!(rendered.ends_with("\n    --- a\n    +++ b"), "{rendered}");
}

#[rstest]
#[case(0, "")]
#[case(3, " and 3 the probe could not read")]
fn unread_names_the_count_only_where_a_file_went_unread(
    #[case] count: usize,
    #[case] clause: &str,
) {
    assert_eq!(unread(count, 1050, "probe"), clause);
}
