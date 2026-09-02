//! Recursive path discovery for the `check` and `format` subcommands.
//!
//! Wraps `ignore::Walk`, honoring `.gitignore`, `.ignore`, the
//! user's global ignore file, and hidden-file conventions. Yields
//! Python source files (`.py`, `.pyi`, `.pyw`) and Jupyter notebooks
//! (`.ipynb`) under the input paths, each paired with its source type.

use std::path::PathBuf;

use ignore::Walk;
use ruff_python_ast::PySourceType;

/// One entry a walk yields.
pub(crate) enum Found {
    /// A file to format, paired with its `PySourceType`.
    Formattable(PathBuf, PySourceType),
    /// A Python-named symlink the walk passed over. Following one
    /// reaches a file outside the tree the caller named, so the walk
    /// leaves it and the runner says so.
    PassedLink(PathBuf),
}

/// Walks `paths` recursively and yields the formattable files under
/// them, each paired with its `PySourceType`, alongside any Python-named
/// symlink it passed over.
///
/// `paths` may contain directories or individual files. Regular files
/// are yielded only when `PySourceType` classifies them as Python
/// source or a notebook. Returns an empty iterator when `paths` is
/// empty.
pub(crate) fn walk(
    paths: &[PathBuf],
) -> impl Iterator<Item = Result<Found, ignore::Error>> + Send + use<> {
    Walk::from_iter(paths).filter_map(|entry| {
        entry
            .map(|e| {
                let source_type = PySourceType::try_from_path(e.path())?;
                let file_type = e.file_type()?;
                if file_type.is_file() {
                    Some(Found::Formattable(e.into_path(), source_type))
                } else if file_type.is_symlink() {
                    Some(Found::PassedLink(e.into_path()))
                } else {
                    None
                }
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
    use crate::testing::formattable;

    fn collect(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
        formattable(paths).into_iter().collect()
    }

    fn passed_links(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
        walk(paths)
            .filter_map(|r| match r.expect("walk entry") {
                Found::PassedLink(path) => Some(path),
                Found::Formattable(..) => None,
            })
            .collect()
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
    fn yields_python_and_notebook_files_and_skips_others() {
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
            BTreeSet::from([
                root.join("a.py"),
                root.join("b.pyi"),
                root.join("c.pyw"),
                root.join("d.ipynb"),
            ])
        );
    }

    #[test]
    fn pairs_each_file_with_its_source_type() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("mod.py"), "");
        write(&root.join("nb.ipynb"), "");

        let types: BTreeSet<_> = walk(&[root.to_path_buf()])
            .map(|r| {
                let Found::Formattable(path, source_type) = r.expect("walk entry") else {
                    unreachable!("invariant: the tree holds no symlink");
                };
                (path.extension().unwrap().to_owned(), source_type.is_ipynb())
            })
            .collect();

        assert_eq!(
            types,
            BTreeSet::from([("py".into(), false), ("ipynb".into(), true)])
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

    #[cfg(unix)]
    #[test]
    fn walk_reports_a_python_symlink_it_passed_over() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("real.py"), "");
        std::os::unix::fs::symlink(root.join("real.py"), root.join("link.py")).expect("links");

        assert_eq!(
            passed_links(&[root.to_path_buf()]),
            BTreeSet::from([root.join("link.py")])
        );
        assert_eq!(
            collect(&[root.to_path_buf()]),
            BTreeSet::from([root.join("real.py")])
        );
    }
}
