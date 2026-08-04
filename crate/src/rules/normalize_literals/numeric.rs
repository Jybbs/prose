//! Canonical spelling for a numeric literal.

use std::borrow::Cow;

/// The canonical spelling of `text`, a numeric literal read verbatim
/// from the source. Hex digits go uppercase while the `0x`, `0o`, and
/// `0b` radix markers, the `e` exponent, and the `j` suffix go
/// lowercase, leaving the digits and any `_` separators where they sit.
pub(super) fn canonical_number(text: &str) -> Cow<'_, str> {
    let spelled = match text.as_bytes() {
        [b'0', b'x' | b'X', ..] => format!("0x{}", text[2..].to_ascii_uppercase()),
        [b'0', marker @ (b'b' | b'B' | b'o' | b'O'), ..] => {
            format!("0{}{}", marker.to_ascii_lowercase() as char, &text[2..])
        }
        _ => text.to_ascii_lowercase(),
    };
    if spelled == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(spelled)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use rstest::rstest;

    use super::*;

    #[rstest]
    fn canonical_number_borrows_a_literal_already_canonical(
        #[values(
            "42", "3.14", "1_000", "0xABC", "0o777", "0b1010", "1e5", "1j", ".5", "1."
        )]
        text: &str,
    ) {
        assert_matches!(canonical_number(text), Cow::Borrowed(_));
    }

    #[rstest]
    #[case("0XABC", "0xABC")]
    #[case("0xabc", "0xABC")]
    #[case("0Xdead_beef", "0xDEAD_BEEF")]
    #[case("0O777", "0o777")]
    #[case("0B1010", "0b1010")]
    #[case("1E5", "1e5")]
    #[case("1E-5", "1e-5")]
    #[case("1.5E10", "1.5e10")]
    #[case("1J", "1j")]
    #[case("1.5J", "1.5j")]
    #[case("10E+3J", "10e+3j")]
    fn canonical_number_respells_a_miscased_literal(#[case] text: &str, #[case] expected: &str) {
        assert_eq!(canonical_number(text), expected);
    }
}
