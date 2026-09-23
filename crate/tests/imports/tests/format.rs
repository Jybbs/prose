//! Tests for formatting a copy of the corpus in place, covering the files
//! the pipeline read and rewrote beside the ones it could not read.

use std::collections::BTreeSet;

use prose::{config::Config, pipeline::Pipeline};

use crate::format::format_tree;

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
    assert_eq!(run.rewritten, BTreeSet::from(["loose.py".to_owned()]));
    assert!(run.rejected.is_empty());
    assert_eq!(
        fs_err::read_to_string(dir.path().join("broken.py")).expect("read it back"),
        "def (\n"
    );
}
