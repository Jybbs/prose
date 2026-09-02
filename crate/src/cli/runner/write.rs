//! Commits a rewrite to disk atomically, through a temporary file
//! renamed over the target with its mode kept.

use std::{
    io::{self, Write},
    path::Path,
};

use tempfile::NamedTempFile;

use super::FileOutcome;
use super::process::failed;
use crate::{cache::Rewrite, cli::exit_status::ExitStatus};

pub(super) fn apply_rewrite(path: &Path, outcome: FileOutcome) -> FileOutcome {
    let FileOutcome::Done {
        rewrite: Rewrite::Changed(kind),
        ..
    } = &outcome
    else {
        return outcome;
    };
    if let Err(e) = write_atomic(path, kind.written()) {
        return failed(ExitStatus::ConfigError, e);
    }
    outcome
}

/// Replaces `path`'s contents with `contents` through a temporary file
/// renamed over the target, so a write that fails partway leaves the
/// original intact rather than truncated at its opening byte. `path`
/// resolves through a symlink first, leaving the link in place and
/// rewriting what it points at. Opening the target beforehand holds the
/// permission check a direct write makes, and the temporary takes the
/// target's mode, which a fresh temporary would otherwise narrow to
/// owner-only. Creating that temporary needs write permission on the
/// containing directory, which a direct write does not.
fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let target = fs_err::canonicalize(path)?;
    let permissions = fs_err::OpenOptions::new()
        .write(true)
        .open(&target)?
        .metadata()?
        .permissions();
    let mut temp = NamedTempFile::new_in(target.parent().unwrap_or(&target))?;
    temp.write_all(contents.as_bytes())?;
    temp.as_file().set_permissions(permissions)?;
    temp.persist(&target).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use tempfile::TempDir;

    use super::*;

    #[cfg(unix)]
    #[test]
    fn write_atomic_holds_the_original_where_no_temporary_can_land() {
        use std::{fs::Permissions, os::unix::fs::PermissionsExt};

        let dir = TempDir::new().expect("a temporary directory");
        let file = dir.path().join("t.py");
        fs_err::write(&file, "x = 1\n").expect("seeds the file");
        fs_err::set_permissions(dir.path(), Permissions::from_mode(0o500)).expect("seals the dir");

        let result = write_atomic(&file, "y = 2\n");

        fs_err::set_permissions(dir.path(), Permissions::from_mode(0o700))
            .expect("reopens the dir");
        assert_matches!(result, Err(_));
        assert_eq!(fs_err::read_to_string(&file).expect("reads"), "x = 1\n");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_the_targets_mode() {
        use std::{fs::Permissions, os::unix::fs::PermissionsExt};

        let dir = TempDir::new().expect("a temporary directory");
        let file = dir.path().join("t.py");
        fs_err::write(&file, "x = 1\n").expect("seeds the file");
        fs_err::set_permissions(&file, Permissions::from_mode(0o755)).expect("sets the mode");

        write_atomic(&file, "y = 2\n").expect("writes the file");

        let mode = fs_err::metadata(&file)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        assert_eq!(fs_err::read_to_string(&file).expect("reads"), "y = 2\n");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_rewrites_through_a_symlink_leaving_the_link() {
        let dir = TempDir::new().expect("a temporary directory");
        let target = dir.path().join("real.py");
        let link = dir.path().join("link.py");
        fs_err::write(&target, "x = 1\n").expect("seeds the target");
        std::os::unix::fs::symlink(&target, &link).expect("links to the target");

        write_atomic(&link, "y = 2\n").expect("writes the file");

        assert!(link.is_symlink());
        assert_eq!(fs_err::read_to_string(&target).expect("reads"), "y = 2\n");
    }
}
