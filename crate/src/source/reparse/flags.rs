//! Rewrites a token over a new range, rebuilding the flags it carries.
//!
//! `Token` takes `TokenFlags` to construct and returns none, so a moved
//! token reads its flags back from `string_flags` and its own text.

use ruff_python_ast::{
    StringFlags,
    str::Quote,
    str_prefix::{
        AnyStringPrefix, ByteStringPrefix, FStringPrefix, StringLiteralPrefix, TStringPrefix,
    },
    token::{Token, TokenFlags, TokenKind},
};
use ruff_text_size::{Ranged, TextRange};

/// `token` rewritten over `range`. `text` is the buffer `token`'s own
/// range indexes.
pub(super) fn retargeted(token: Token, text: &str, range: TextRange) -> Token {
    Token::new(token.kind(), range, flags_of(token, text))
}

/// The flags `token` carries, read from its string flags where it is a
/// string and from its own text where it is a name.
fn flags_of(token: Token, text: &str) -> TokenFlags {
    let Some(string) = token.string_flags() else {
        return if token.kind() == TokenKind::Name && !text[token.range()].is_ascii() {
            TokenFlags::NON_ASCII_NAME
        } else {
            TokenFlags::empty()
        };
    };
    let mut flags = prefix_flags(string.prefix());
    if string.quote_style() == Quote::Double {
        flags |= TokenFlags::DOUBLE_QUOTES;
    }
    if string.is_triple_quoted() {
        flags |= TokenFlags::TRIPLE_QUOTED_STRING;
    }
    if string.is_unclosed() {
        flags |= TokenFlags::UNCLOSED_STRING;
    }
    flags
}

/// The family and raw-case bits `prefix` spells.
fn prefix_flags(prefix: AnyStringPrefix) -> TokenFlags {
    use AnyStringPrefix::{Bytes, Format, Regular, Template};

    let family = match prefix {
        Bytes(_) => TokenFlags::BYTE_STRING,
        Format(_) => TokenFlags::F_STRING,
        Template(_) => TokenFlags::T_STRING,
        Regular(StringLiteralPrefix::Unicode) => TokenFlags::UNICODE_STRING,
        Regular(_) => TokenFlags::empty(),
    };
    let raw = match prefix {
        Bytes(ByteStringPrefix::Raw { uppercase_r: upper })
        | Format(FStringPrefix::Raw { uppercase_r: upper })
        | Template(TStringPrefix::Raw { uppercase_r: upper })
        | Regular(StringLiteralPrefix::Raw { uppercase: upper }) => {
            if upper {
                TokenFlags::RAW_STRING_UPPERCASE
            } else {
                TokenFlags::RAW_STRING_LOWERCASE
            }
        }
        _ => TokenFlags::empty(),
    };
    family | raw
}

#[cfg(test)]
mod tests {
    use ruff_python_parser::parse_module;

    use super::*;

    /// A module carrying one token of every flag-bearing shape, covering
    /// each string prefix in both raw cases, both quote styles, triple
    /// quoting, and a name outside ASCII.
    const FLAVORS: &str = r#"
plain = "double"
single = 'single'
triple = """three"""
triple_single = '''three'''
raw = r"raw"
raw_upper = R"raw"
unicode = u"unicode"
byte = b"bytes"
byte_raw = rb"bytes"
byte_raw_upper = Rb"bytes"
formatted = f"{plain}"
formatted_raw = rf"{plain}"
formatted_raw_upper = Rf"{plain}"
templated = t"{plain}"
templated_raw = rt"{plain}"
templated_raw_upper = Rt"{plain}"
spec = f"{plain:>{raw}}"
ünïcode_name = 1
"#;

    #[test]
    fn retargeted_moves_a_token_and_keeps_its_flags() {
        let parsed = parse_module(FLAVORS).expect("the sample parses");
        let string = parsed
            .tokens()
            .iter()
            .find(|token| {
                token
                    .string_flags()
                    .is_some_and(StringFlags::is_triple_quoted)
            })
            .expect("the sample carries a triple-quoted string");
        let moved = retargeted(*string, FLAVORS, TextRange::new(0.into(), 1.into()));

        assert_eq!(moved.range(), TextRange::new(0.into(), 1.into()));
        assert_eq!(moved.string_flags(), string.string_flags());
    }

    #[test]
    fn retargeted_rebuilds_every_flag_a_token_carries() {
        let parsed = parse_module(FLAVORS).expect("the sample parses");
        let mut checked = 0;
        for token in parsed.tokens().iter() {
            assert_eq!(
                retargeted(*token, FLAVORS, token.range()),
                *token,
                "token at {:?} lost a flag",
                token.range(),
            );
            checked += 1;
        }
        assert!(checked > 60, "the sample exercises the flag-bearing kinds");
    }
}
