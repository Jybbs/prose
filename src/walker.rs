//! Recursive path discovery for the `check` and `format` subcommands.
//!
//! Wraps `ignore::WalkBuilder` with the defaults prose wants. Honors
//! `.gitignore`, `.ignore`, the user's global ignore file, and skips
//! hidden files. Yields the Python source files under the input paths,
//! using `ruff_python_ast::PySourceType` to classify extensions so
//! prose covers `.py`, `.pyi`, and `.pyw` in lockstep with the rest of
//! the ruff ecosystem. The walker is a serial iterator, leaving rayon
//! to attach downstream via `par_bridge`.

use std::path::PathBuf;

use ignore::WalkBuilder;
use ruff_python_ast::PySourceType;

/// Walks `paths` recursively and yields the Python files under them.
///
/// `paths` may contain directories or individual files. Directories are
/// traversed, whereas regular files are yielded only when `PySourceType`
/// classifies them as Python source (`.py`, `.pyi`, `.pyw`). Entries
/// are filtered through the `ignore` crate's default ignore-file stack,
/// so `.gitignore`, `.ignore`, the global ignore config, and
/// hidden-file conventions are honored automatically.
///
/// Returns an empty iterator when `paths` is empty, so the caller does
/// not have to special-case that branch.
pub fn walk(paths: &[PathBuf]) -> impl Iterator<Item = Result<PathBuf, ignore::Error>> + Send {
    let builder = paths.split_first().map(|(first, rest)| {
        let mut b = WalkBuilder::new(first);
        for path in rest {
            b.add(path);
        }
        b
    });

    builder
        .into_iter()
        .flat_map(|b| b.build())
        .filter_map(|entry| {
            entry
                .map(|e| {
                    let is_python_file = e.file_type().is_some_and(|ft| ft.is_file())
                        && PySourceType::try_from_path(e.path())
                            .is_some_and(|ty| ty.is_py_file_or_stub());
                    is_python_file.then(|| e.into_path())
                })
                .transpose()
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn collect(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
        walk(paths).map(|r| r.expect("walk entry")).collect()
    }

    fn write(path: &std::path::Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, contents).expect("write file");
    }

    #[test]
    fn empty_input_yields_empty_iterator() {
        let results: Vec<_> = walk(&[]).collect();
        assert!(results.is_empty());
    }

    #[test]
    fn honors_ignore_files() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join(".ignore"), "skip/\n");
        write(&root.join("skip/ignored.py"), "");
        write(&root.join("kept.py"), "");

        let found = collect(&[root.to_path_buf()]);

        assert_eq!(found, BTreeSet::from([root.join("kept.py")]));
    }

    #[test]
    fn merges_multiple_input_roots() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("a/one.py"), "");
        write(&root.join("b/two.py"), "");

        let found = collect(&[root.join("a"), root.join("b")]);

        assert_eq!(
            found,
            BTreeSet::from([root.join("a/one.py"), root.join("b/two.py")])
        );
    }

    #[test]
    fn skips_hidden_directories_by_default() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join(".hidden/foo.py"), "");
        write(&root.join("visible/bar.py"), "");

        let found = collect(&[root.to_path_buf()]);

        assert_eq!(found, BTreeSet::from([root.join("visible/bar.py")]));
    }

    #[test]
    fn yields_python_source_files_and_skips_others() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("a.py"), "");
        write(&root.join("b.pyi"), "");
        write(&root.join("c.pyw"), "");
        write(&root.join("d.ipynb"), "");
        write(&root.join("e.txt"), "");
        write(&root.join("f.md"), "");

        let found = collect(&[root.to_path_buf()]);

        assert_eq!(
            found,
            BTreeSet::from([root.join("a.py"), root.join("b.pyi"), root.join("c.pyw"),])
        );
    }

    #[test]
    fn yields_single_file_when_path_is_a_file() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("lone.py");
        write(&file, "x = 1\n");

        let found = collect(std::slice::from_ref(&file));

        assert_eq!(found, BTreeSet::from([file]));
    }
}
