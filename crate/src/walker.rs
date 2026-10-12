//! Recursive path discovery for the `check` and `format` subcommands.
//!
//! Wraps `ignore::Walk`, honoring `.gitignore`, `.ignore`, the
//! user's global ignore file, and hidden-file conventions. Yields
//! Python source files (`.py`, `.pyi`, `.pyw`) and Jupyter notebooks
//! (`.ipynb`) under the input paths, each paired with its source type,
//! and rejects a named path that is neither a directory nor one of them.

use std::path::PathBuf;

use ignore::{DirEntry, Walk};
use ruff_python_ast::PySourceType;
use rustc_hash::FxHashSet;

/// One entry a walk yields.
#[derive(Debug, PartialEq)]
pub(crate) enum Found {
    /// A file to format, paired with its `PySourceType`.
    Formattable(PathBuf, PySourceType),
    /// A Python-named symlink the walk passed over and the runner
    /// reports.
    PassedLink(PathBuf),
    /// A path named in the input that is neither a directory nor a file
    /// the walk formats, which the runner reports as an error.
    Rejected(PathBuf),
}

impl Found {
    /// Classifies `entry`, returning `None` for one the walk passes over
    /// silently. A named path that resolves to a directory, through a
    /// symlink or not, is a root the walk descends rather than a rejection.
    fn from_entry(entry: DirEntry) -> Option<Self> {
        let is_file = entry.file_type().is_some_and(|t| t.is_file());
        match PySourceType::try_from_path(entry.path()) {
            Some(source_type) if is_file => Some(Self::Formattable(entry.into_path(), source_type)),
            _ if entry.depth() == 0 => {
                (!entry.path().is_dir()).then(|| Self::Rejected(entry.into_path()))
            }
            Some(_) if entry.path_is_symlink() => Some(Self::PassedLink(entry.into_path())),
            _ => None,
        }
    }
}

/// Walks `paths` recursively and yields the formattable files under
/// them, each paired with its `PySourceType`, alongside any Python-named
/// symlink it passed over and any named path it rejected.
///
/// `paths` may contain directories or individual files. Regular files
/// are yielded only when `PySourceType` classifies them as Python
/// source or a notebook. A file or a rejected path reachable twice is
/// yielded once. Returns an empty iterator when `paths` is empty.
pub(crate) fn walk(
    paths: &[PathBuf],
) -> impl Iterator<Item = Result<Found, ignore::Error>> + Send + use<> {
    let mut seen = FxHashSet::default();
    Walk::from_iter(paths).filter_map(move |entry| {
        let found = entry.map(Found::from_entry).transpose()?;
        match &found {
            Ok(Found::Formattable(path, _) | Found::Rejected(path))
                if !seen.insert(path.clone()) =>
            {
                None
            }
            _ => Some(found),
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;
    use crate::testing::formattable;

    fn collect(paths: &[PathBuf]) -> BTreeSet<PathBuf> {
        formattable(paths).into_iter().collect()
    }

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, contents).expect("write file");
    }

    fn yielded(paths: &[PathBuf]) -> Vec<Found> {
        walk(paths).map(|r| r.expect("walk entry")).collect()
    }

    #[cfg(unix)]
    #[test]
    fn classifies_a_named_symlink_by_its_own_name() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("module.py"), "");
        write(&root.join("script"), "");
        std::os::unix::fs::symlink(root.join("module.py"), root.join("link")).expect("links");
        std::os::unix::fs::symlink(root.join("script"), root.join("alias.py")).expect("links");
        let named = [root.join("link"), root.join("alias.py")];

        assert_eq!(
            yielded(&named),
            [
                Found::Rejected(root.join("link")),
                Found::Formattable(root.join("alias.py"), PySourceType::Python),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn descends_a_named_python_symlink_to_a_directory_without_a_note() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("pkg/mod.py"), "");
        std::os::unix::fs::symlink(root.join("pkg"), root.join("pkg.py")).expect("links");
        let named = [root.join("pkg.py")];

        assert_eq!(
            yielded(&named),
            [Found::Formattable(
                root.join("pkg.py/mod.py"),
                PySourceType::Python
            )]
        );
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
    fn pairs_each_file_with_its_source_type() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("mod.py"), "");
        write(&root.join("nb.ipynb"), "");

        let types: BTreeSet<_> = yielded(&[root.to_path_buf()])
            .into_iter()
            .map(|found| {
                let Found::Formattable(path, source_type) = found else {
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
    fn passes_over_a_file_beneath_a_named_directory_silently() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("script"), "");
        write(&root.join("kept.py"), "");

        assert_eq!(
            yielded(&[root.to_path_buf()]),
            [Found::Formattable(
                root.join("kept.py"),
                PySourceType::Python
            )]
        );
    }

    #[rstest]
    fn rejects_a_named_file_outside_the_python_extensions(
        #[values("script", "notes.txt", "module.PY")] name: &str,
    ) {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join(name);
        write(&file, "x = 1\n");

        assert_eq!(
            yielded(std::slice::from_ref(&file)),
            [Found::Rejected(file)]
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_named_python_socket() {
        let tmp = TempDir::new().expect("tempdir");
        let socket = tmp.path().join("sock.py");
        let _listener = std::os::unix::net::UnixListener::bind(&socket).expect("binds");

        assert_eq!(
            yielded(std::slice::from_ref(&socket)),
            [Found::Rejected(socket)]
        );
    }

    #[test]
    fn rejects_a_path_named_twice_once() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("script");
        write(&file, "");

        assert_eq!(
            yielded(&[file.clone(), file.clone()]),
            [Found::Rejected(file)]
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

    #[cfg(unix)]
    #[test]
    fn walk_reports_a_python_symlink_it_passed_over() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("real.py"), "");
        std::os::unix::fs::symlink(root.join("real.py"), root.join("link.py")).expect("links");

        let found = yielded(&[root.to_path_buf()]);

        assert_eq!(found.len(), 2);
        assert!(found.contains(&Found::PassedLink(root.join("link.py"))));
        assert!(found.contains(&Found::Formattable(
            root.join("real.py"),
            PySourceType::Python
        )));
    }

    #[test]
    fn yields_a_file_reachable_from_two_roots_once() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        write(&root.join("mod.py"), "");

        assert_eq!(
            yielded(&[root.to_path_buf(), root.join("mod.py")]),
            [Found::Formattable(
                root.join("mod.py"),
                PySourceType::Python
            )]
        );
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
    fn yields_single_file_when_path_is_a_file() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("lone.py");
        write(&file, "x = 1\n");

        let found = collect(std::slice::from_ref(&file));

        assert_eq!(found, BTreeSet::from([file]));
    }
}
