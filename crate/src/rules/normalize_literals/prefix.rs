//! Canonical spelling for a string literal's prefix.

use ruff_python_ast::str_prefix::AnyStringPrefix;

/// The canonical spelling of `prefix`, every letter lowercase and the
/// no-op `u` dropped. The parsed prefix already collapses letter order,
/// so `BR` and `rB` both reach `rb`.
pub(super) fn canonical_prefix(prefix: AnyStringPrefix) -> &'static str {
    match (prefix, prefix.is_raw()) {
        (AnyStringPrefix::Bytes(_), false) => "b",
        (AnyStringPrefix::Bytes(_), true) => "rb",
        (AnyStringPrefix::Format(_), false) => "f",
        (AnyStringPrefix::Format(_), true) => "rf",
        (AnyStringPrefix::Template(_), false) => "t",
        (AnyStringPrefix::Template(_), true) => "rt",
        (AnyStringPrefix::Regular(_), false) => "",
        (AnyStringPrefix::Regular(_), true) => "r",
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_python_ast::str_prefix::{
        ByteStringPrefix, FStringPrefix, StringLiteralPrefix, TStringPrefix,
    };

    use super::*;

    #[rstest]
    #[case(AnyStringPrefix::Regular(StringLiteralPrefix::Empty), "")]
    #[case(AnyStringPrefix::Regular(StringLiteralPrefix::Unicode), "")]
    #[case(AnyStringPrefix::Regular(StringLiteralPrefix::Raw { uppercase: false }), "r")]
    #[case(AnyStringPrefix::Regular(StringLiteralPrefix::Raw { uppercase: true }), "r")]
    #[case(AnyStringPrefix::Bytes(ByteStringPrefix::Regular), "b")]
    #[case(AnyStringPrefix::Bytes(ByteStringPrefix::Raw { uppercase_r: true }), "rb")]
    #[case(AnyStringPrefix::Format(FStringPrefix::Regular), "f")]
    #[case(AnyStringPrefix::Format(FStringPrefix::Raw { uppercase_r: true }), "rf")]
    #[case(AnyStringPrefix::Template(TStringPrefix::Regular), "t")]
    #[case(AnyStringPrefix::Template(TStringPrefix::Raw { uppercase_r: false }), "rt")]
    fn canonical_prefix_lowercases_every_letter_and_drops_unicode(
        #[case] prefix: AnyStringPrefix,
        #[case] expected: &str,
    ) {
        assert_eq!(canonical_prefix(prefix), expected);
    }
}
