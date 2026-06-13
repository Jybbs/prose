//! Mapping between filesystem paths and `file://` URIs.

use std::path::{Path, PathBuf};

use fluent_uri::pct_enc::{EString, encoder::Path as PathEncoder};
use lsp_types::Uri;

const SCHEME: &str = "file";

/// Renders an absolute source path as a percent-encoded `file://` URI,
/// passing a relative path through unchanged.
pub(crate) fn from_path(name: &str) -> String {
    if Path::new(name).is_absolute() {
        let mut encoded = EString::<PathEncoder>::new();
        encoded.encode_str::<PathEncoder>(name);
        format!("{SCHEME}://{encoded}")
    } else {
        name.to_owned()
    }
}

/// Turns a `file://` URI into a filesystem path, or `None` for a URI that
/// names no local file.
pub(crate) fn to_path(uri: &Uri) -> Option<PathBuf> {
    if uri.scheme().map(|scheme| scheme.as_str()) != Some(SCHEME) {
        return None;
    }
    let decoded = uri.path().as_estr().decode().into_string().ok()?;
    // A Windows drive arrives as `/C:/dir`, so drop its leading slash.
    let path = match decoded.strip_prefix('/') {
        Some(rest) if has_drive_prefix(rest) => rest,
        _ => decoded.as_ref(),
    };
    Some(PathBuf::from(path))
}

/// Returns `true` when `s` opens with a `C:`-style Windows drive letter.
fn has_drive_prefix(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::uri;

    #[test]
    fn from_path_passes_a_relative_path_through_unchanged() {
        assert_eq!(from_path("src/My Project/mod.py"), "src/My Project/mod.py");
    }

    #[rstest]
    #[case("/tmp/My Project/mod.py", "file:///tmp/My%20Project/mod.py")]
    #[case("/tmp/café/mod.py", "file:///tmp/caf%C3%A9/mod.py")]
    #[case("/tmp/a#b?c[d]/mod.py", "file:///tmp/a%23b%3Fc%5Bd%5D/mod.py")]
    fn from_path_round_trips_an_absolute_path(#[case] path: &str, #[case] expected: &str) {
        let encoded = from_path(path);
        assert_eq!(encoded, expected);
        assert_eq!(to_path(&uri(&encoded)), Some(PathBuf::from(path)));
    }

    #[test]
    fn to_path_decodes_percent_escapes() {
        assert_eq!(
            to_path(&uri("file:///tmp/a%20b.py")),
            Some(PathBuf::from("/tmp/a b.py")),
        );
    }

    #[test]
    fn to_path_rejects_a_non_file_scheme() {
        assert!(to_path(&uri("untitled:Untitled-1")).is_none());
    }

    #[test]
    fn to_path_strips_a_windows_drive_slash() {
        assert_eq!(
            to_path(&uri("file:///C:/Users/x.py")),
            Some(PathBuf::from("C:/Users/x.py")),
        );
    }
}
