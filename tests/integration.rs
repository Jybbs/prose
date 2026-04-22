//! Integration tests exercising each rule against golden-file fixtures.
//!
//! Each rule owns a subdirectory under `tests/fixtures/<rule>/`
//! containing one or more `<case>.input.py` files. The expected
//! transform output is an `insta` snapshot under `tests/snapshots/`
//! that `cargo insta review` manages.
//!
//! The `apply` dispatch below maps a fixture directory name to the
//! transform to run. `identity` exists as the seed no-op used to prove
//! the harness works end-to-end; further arms land as each rule issue
//! merges.

use std::ffi::OsStr;

use prose::source::Source;

fn apply(rule: &str, source: &Source) -> String {
    match rule {
        "identity" => source.text().to_owned(),
        other => panic!("no transform wired for fixture directory `{other}`"),
    }
}

#[test]
fn fixtures() {
    insta::glob!("fixtures/**/*.input.py", |path| {
        let rule = path
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .expect("fixture path has a parent directory name");

        let source = Source::from_path(path).expect("fixture input reads and parses as Python");

        insta::assert_snapshot!(apply(rule, &source));
    });
}
