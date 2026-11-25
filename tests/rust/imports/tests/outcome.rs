//! Tests for the record `probe.py` writes and the harness reads back,
//! covering its tagged rows and the module paths a run names.

use std::{collections::BTreeSet, path::Path};

use crate::outcome::{Kind, Outcome, Raise, relative_to};

#[test]
fn a_name_set_aside_leaves_every_record_of_it_and_nothing_else() {
    let ran = Outcome {
        constants: [
            ("KEPT".to_owned(), "1".to_owned()),
            ("VARIES".to_owned(), "0x7f".to_owned()),
        ]
        .into(),
        kind: Kind::Ok,
        names: vec!["KEPT".to_owned(), "VARIES".to_owned(), "other".to_owned()],
        unevaluated: [("KEPT.m", "NameError"), ("VARIES", "NameError")]
            .map(|(held, raised)| {
                let raise = Raise {
                    missing: None,
                    raised: raised.to_owned(),
                };
                (held.to_owned(), raise)
            })
            .into(),
        ..Outcome::default()
    };
    let kept = ran.without(&["VARIES".to_owned()].into());
    assert_eq!(kept.names, ["KEPT", "other"]);
    assert_eq!(kept.constants, [("KEPT".to_owned(), "1".to_owned())].into());
    assert_eq!(kept.unevaluated.keys().collect::<Vec<_>>(), ["KEPT.m"]);
    assert_eq!(kept.kind, Kind::Ok);
    assert_eq!(ran.without(&BTreeSet::new()).names, ran.names);
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
    assert_eq!(read.importing, None);
    assert_eq!(read.name, Some("x".to_owned()));
    assert_eq!(read.raised, "NameError");
}

#[test]
fn an_importing_row_names_the_module_a_failed_import_named() {
    let record = [
        ["kind", "raised"].join("\0"),
        ["importing", "pkg.mod"].join("\0"),
    ]
    .join("\u{1e}");
    let read = Outcome::parse(&record, &[]);
    assert_eq!(read.importing, Some("pkg.mod".to_owned()));
}

#[test]
fn an_unevaluated_row_names_the_definition_beside_what_it_raised() {
    let record = [
        ["kind", "ok"].join("\0"),
        ["unevaluated", "C.m", "NameError", "Sequence"].join("\0"),
        ["unevaluated", "f", "ValueError", ""].join("\0"),
    ]
    .join("\u{1e}");
    let read = Outcome::parse(&record, &[]);
    assert_eq!(
        read.unevaluated["C.m"],
        Raise {
            missing: Some("Sequence".to_owned()),
            raised: "NameError".to_owned(),
        }
    );
    assert_eq!(read.unevaluated["f"].missing, None);
}

#[test]
fn an_unrecognized_kind_row_reads_as_unmeasured() {
    let read = Outcome::parse(&["kind", "wat"].join("\0"), &[]);
    assert_eq!(read.kind, Kind::Unmeasured);
}

#[test]
fn parsing_a_record_filters_loader_names_and_reads_frames() {
    let record = [
        ["kind", "ok"].join("\0"),
        ["bound", "__file__"].join("\0"),
        ["bound", "__annotate__"].join("\0"),
        ["bound", "__all__"].join("\0"),
        ["bound", "N"].join("\0"),
        ["const", "__all__", "('a',)"].join("\0"),
        ["const", "__annotations__", "('N',)"].join("\0"),
        ["const", "N", "1"].join("\0"),
        ["frame", "9", "/tree/m.py"].join("\0"),
        ["loaded", "/tree/m.py"].join("\0"),
        ["loaded", "/elsewhere/other.py"].join("\0"),
    ]
    .join("\u{1e}");
    let read = Outcome::parse(&record, &[Path::new("/tree")]);
    assert_eq!(read.kind, Kind::Ok);
    assert_eq!(read.names, ["N", "__all__"]);
    assert_eq!(
        read.constants,
        [
            ("N".to_owned(), "1".to_owned()),
            ("__annotations__".to_owned(), "('N',)".to_owned()),
        ]
        .into()
    );
    assert_eq!(read.frames, [("/tree/m.py".to_owned(), 9)]);
    assert_eq!(read.loaded, ["m.py"]);
}
