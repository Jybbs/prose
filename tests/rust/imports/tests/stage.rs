//! Tests for the stage a sweep works in, covering the overlay a single-rule
//! re-format runs over.

use crate::stage::Stage;

#[test]
fn an_overlay_holds_the_top_level_package_and_goes_once_it_drops() {
    let corpus = tempfile::tempdir().expect("a scratch corpus");
    for (path, text) in [
        ("other.py", ""),
        ("pkg/__init__.py", ""),
        ("pkg/mod.py", "VALUE = 1\n"),
    ] {
        let file = corpus.path().join(path);
        fs_err::create_dir_all(file.parent().expect("a file sits in a directory"))
            .expect("create a corpus directory");
        fs_err::write(file, text).expect("write a corpus file");
    }
    let stage = Stage::new(corpus.path());
    let overlay = stage.overlay(
        &["pkg/mod.py".to_owned()],
        "default",
        "pkg/mod.py",
        "prefer-fstring",
    );
    let tree = overlay.path().to_path_buf();
    assert!(tree.join("pkg/__init__.py").is_file());
    assert!(!tree.join("other.py").exists());
    drop(overlay);
    assert!(!tree.exists());
}
