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

/// Whether a buffer's names can reach past ASCII at all, read once so a
/// token of an all-ASCII buffer skips the per-name scan.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Naming {
    Ascii,
    Wide,
}

impl Naming {
    pub(super) fn of(text: &str) -> Self {
        if text.is_ascii() {
            Self::Ascii
        } else {
            Self::Wide
        }
    }
}

/// `token` rewritten over `range`. `text` is the buffer `token`'s own
/// range indexes.
pub(super) fn retargeted(token: Token, named: Naming, text: &str, range: TextRange) -> Token {
    Token::new(token.kind(), range, flags_of(token, named, text))
}

/// The flags `token` carries, read from its string flags where it is a
/// string and from its own text where it is a name.
fn flags_of(token: Token, named: Naming, text: &str) -> TokenFlags {
    let Some(string) = token.string_flags() else {
        let wide_name = named == Naming::Wide
            && token.kind() == TokenKind::Name
            && !text[token.range()].is_ascii();
        return if wide_name {
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
    match prefix {
        Bytes(ByteStringPrefix::Raw { uppercase_r: true })
        | Format(FStringPrefix::Raw { uppercase_r: true })
        | Template(TStringPrefix::Raw { uppercase_r: true })
        | Regular(StringLiteralPrefix::Raw { uppercase: true }) => {
            family | TokenFlags::RAW_STRING_UPPERCASE
        }
        Bytes(ByteStringPrefix::Raw { uppercase_r: false })
        | Format(FStringPrefix::Raw { uppercase_r: false })
        | Template(TStringPrefix::Raw { uppercase_r: false })
        | Regular(StringLiteralPrefix::Raw { uppercase: false }) => {
            family | TokenFlags::RAW_STRING_LOWERCASE
        }
        _ => family,
    }
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
        let moved = retargeted(
            *string,
            Naming::of(FLAVORS),
            FLAVORS,
            TextRange::new(0.into(), 1.into()),
        );

        assert_eq!(moved.range(), TextRange::new(0.into(), 1.into()));
        assert_eq!(moved.string_flags(), string.string_flags());
    }

    #[test]
    fn retargeted_rebuilds_every_flag_a_token_carries() {
        let parsed = parse_module(FLAVORS).expect("the sample parses");
        let mut checked = 0;
        for token in parsed.tokens().iter() {
            assert_eq!(
                retargeted(*token, Naming::of(FLAVORS), FLAVORS, token.range()),
                *token,
                "token at {:?} lost a flag",
                token.range(),
            );
            checked += 1;
        }
        assert!(checked > 60, "the sample exercises the flag-bearing kinds");
    }
}
