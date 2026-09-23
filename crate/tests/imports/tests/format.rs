//! Tests for formatting a copy of the corpus in place, covering the files the
//! pipeline read and rewrote beside one it could not parse, and the rows an
//! edit rewrote.

use std::collections::BTreeSet;

use prose::{config::Config, pipeline::Pipeline};
use ruff_source_file::LineIndex;

use crate::format::{edit_rows, format_tree, row_of};

#[test]
fn a_file_that_does_not_parse_is_counted_and_left_out() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let write = |name: &str, text: &str| {
        fs_err::write(dir.path().join(name), text).expect("write a corpus file");
    };
    write("kept.py", "x = 1\n");
    write("loose.py", "x=1\n");
    write("broken.py", "def (\n");
    let run = format_tree(dir.path(), &Pipeline::with_defaults(&Config::default()));
    assert_eq!(run.unread, 1);
    assert_eq!(
        run.read,
        BTreeSet::from(["kept.py".to_owned(), "loose.py".to_owned()])
    );
    assert_eq!(run.rewritten, 1);
    assert!(run.rejected.is_empty());
    assert_eq!(
        fs_err::read_to_string(dir.path().join("broken.py")).expect("read it back"),
        "def (\n"
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
fn rows_count_from_one() {
    let lines = LineIndex::from_source_text("a\nbb\nccc\n");
    assert_eq!(row_of(&lines, 0), 1);
    assert_eq!(row_of(&lines, 2), 2);
    assert_eq!(row_of(&lines, 5), 3);
}
