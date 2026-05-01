//! Shared snapshot harness for integration test binaries.

use std::ffi::OsStr;
use std::path::Path;

use similar::TextDiff;

pub fn case_stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(OsStr::to_str)
        .expect("fixture path has a stem")
}

pub fn in_snapshot_dir(directory: &str, f: impl FnOnce()) {
    insta::with_settings!({
        snapshot_path => format!("snapshots/{directory}"),
        prepend_module_to_snapshot => false,
        snapshot_suffix => "",
    }, {
        f();
    });
}

pub fn unified_diff(expected: &str, actual: &str) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("expected", "actual")
        .to_string()
}
