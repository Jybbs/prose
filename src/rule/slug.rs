//! Compile-time kebab-slug validators used by the registry's
//! cross-row assertions.

/// Returns `true` when `bytes` is a valid kebab-case slug. Non-empty,
/// starts and ends with a lowercase ASCII letter or digit, contains
/// only lowercase ASCII letters, digits, and dashes, and has no `--`
/// substring.
pub(super) const fn is_valid_slug(bytes: &[u8]) -> bool {
    let mut i = 0;
    let mut prev_was_dash = true;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'-' {
            if prev_was_dash {
                return false;
            }
            prev_was_dash = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_was_dash = false;
        } else {
            return false;
        }
        i += 1;
    }
    !prev_was_dash
}

/// Byte-wise equality on `&[u8]` usable from const contexts.
pub(super) const fn slug_bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn is_valid_slug_accepts_canonical_kebab_shapes(
        #[values("a", "a-b", "abc123", "single-use-variables")] valid: &str,
    ) {
        assert!(is_valid_slug(valid.as_bytes()));
    }

    #[rstest]
    fn is_valid_slug_rejects_invalid_shapes(
        #[values("", "-foo", "foo-", "a--b", "Foo", "abc!")] invalid: &str,
    ) {
        assert!(!is_valid_slug(invalid.as_bytes()));
    }

    #[test]
    fn slug_bytes_equal_matches_only_identical_slices() {
        assert!(slug_bytes_equal(b"foo", b"foo"));
        assert!(!slug_bytes_equal(b"foo", b"food"));
        assert!(!slug_bytes_equal(b"foo", b"bar"));
    }
}
