//! Renders filesystem paths as `file://` URIs.

use std::path::Path;

use fluent_uri::pct_enc::{EString, encoder::Path as PathEncoder};

pub(crate) const SCHEME: &str = "file";

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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn from_path_passes_a_relative_path_through_unchanged() {
        assert_eq!(from_path("src/My Project/mod.py"), "src/My Project/mod.py");
    }

    #[rstest]
    #[case("/tmp/My Project/mod.py", "file:///tmp/My%20Project/mod.py")]
    #[case("/tmp/café/mod.py", "file:///tmp/caf%C3%A9/mod.py")]
    #[case("/tmp/a#b?c[d]/mod.py", "file:///tmp/a%23b%3Fc%5Bd%5D/mod.py")]
    fn from_path_percent_encodes_an_absolute_path(#[case] path: &str, #[case] expected: &str) {
        assert_eq!(from_path(path), expected);
    }
}
