//! Tests for the span and row arithmetic behind an attribution, covering
//! whether a fix reached a given row, which rows an edit rewrote, and the
//! hunk a report shows around them.

use std::ops::Range;

use ruff_source_file::LineIndex;
use similar::TextDiff;

use crate::{
    diff::{hunk, mapped_rows},
    fixes::{drops, holds_word, reaches, rewritten},
    format::{edit_rows, row_of},
    records::EditRows,
};

/// Ten lines `l1` through `l10`, the text the hunk tests rewrite.
const LINES: [&str; 10] = ["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10"];

/// One edit rewriting `range` of `text` with `content`.
fn edit(content: &str, range: Range<usize>, text: &str) -> EditRows {
    EditRows {
        content: content.to_owned(),
        rows: edit_rows(&LineIndex::from_source_text(text), text, &range),
        range,
    }
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
fn dropping_a_name_reads_whole_words_only() {
    let text = "from m import a, b\n";
    let edits = [edit("from m import a", 0..18, text)];
    assert!(drops(&edits, "b", text));
    assert!(!drops(&edits, "a", text));
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
fn the_hunk_centres_on_the_changed_line_naming_the_name() {
    let mut after = LINES;
    after[2] = "L3";
    after[5] = "MARK";
    let lines = hunk(&TextDiff::from_slices(&LINES, &after), None, "MARK");
    assert!(lines.iter().any(|line| line == "+MARK"));
    assert!(!lines.iter().any(|line| line == "+L3"));
}

#[test]
fn the_hunk_cuts_context_either_side_of_the_row() {
    let mut after = LINES;
    after[5] = "L6";
    assert_eq!(
        hunk(&TextDiff::from_slices(&LINES, &after), Some(6), ""),
        [
            "...", " l4", " l5", "-l6", "+L6", " l7", " l8", " l9", "..."
        ]
    );
}

#[test]
fn the_hunk_falls_back_to_the_first_change_with_no_row_or_name() {
    let mut after = LINES;
    after[6] = "L7";
    let lines = hunk(&TextDiff::from_slices(&LINES, &after), None, "");
    assert!(lines.iter().any(|line| line == "-l7"), "{lines:?}");
    assert!(lines.iter().any(|line| line == "+L7"), "{lines:?}");
}

#[test]
fn whole_word_matching_rejects_a_longer_identifier() {
    assert!(holds_word("from m import a, b", "a"));
    assert!(!holds_word("from m import ab", "a"));
    assert!(!holds_word("renamed", "name"));
}
