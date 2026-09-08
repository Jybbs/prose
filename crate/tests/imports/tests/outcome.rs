//! Tests for the record `probe.py` writes and the harness reads back,
//! covering its tagged rows and the module paths a run names.

use std::path::Path;

use crate::outcome::{Kind, Outcome, relative_to};

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
fn an_unrecognised_kind_row_reads_as_unmeasured() {
    let read = Outcome::parse(&["kind", "wat"].join("\0"), &[]);
    assert_eq!(read.kind, Kind::Unmeasured);
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
