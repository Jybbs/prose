//! Reads a template literal's delimiters and reassembles it as an
//! f-string.

use std::borrow::Cow;

use ruff_python_ast::{
    AnyStringFlags, ExprStringLiteral, StringFlags,
    str_prefix::{AnyStringPrefix, FStringPrefix, StringLiteralPrefix},
};

use crate::source::Source;

/// One template literal, holding the body between its quotes alongside
/// the flags the rewritten f-string carries.
pub(super) struct Template<'src> {
    pub(super) body: &'src str,
    flags: AnyStringFlags,
    pub(super) raw: bool,
}

impl<'src> Template<'src> {
    /// Reads `expr` as a single-part template. `None` covers an
    /// implicitly concatenated literal and a `u` prefix, which an
    /// f-string cannot carry.
    pub(super) fn read(source: &'src Source, expr: &ExprStringLiteral) -> Option<Self> {
        let literal = expr.as_single_part_string()?;
        let flags = literal.flags;
        if flags.prefix() == StringLiteralPrefix::Unicode {
            return None;
        }
        let raw = flags.prefix().is_raw();
        let prefix = AnyStringPrefix::Format(if raw {
            FStringPrefix::Raw { uppercase_r: false }
        } else {
            FStringPrefix::Regular
        });
        Some(Self {
            body: source.slice(literal.content_range()),
            flags: AnyStringFlags::new(prefix, flags.quote_style(), flags.triple_quotes()),
            raw,
        })
    }

    /// `body` wrapped in the f-string prefix and the quotes the source
    /// wrote, so a raw template comes back as `rf"..."`.
    pub(super) fn wrap(&self, body: &str) -> String {
        let quote = self.flags.quote_str();
        format!("{}{quote}{body}{quote}", self.flags.prefix())
    }
}

/// `text` with each brace doubled so an f-string reads it as a literal,
/// leaving a `\N{...}` name escape whole.
pub(super) fn escape_braces(text: &str) -> Cow<'_, str> {
    if !text.contains(['{', '}']) {
        return Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len() + 2);
    let mut rest = text;
    while let Some(offset) = rest.find(['\\', '{', '}']) {
        let (head, tail) = rest.split_at(offset);
        escaped.push_str(head);
        if let Some(len) = name_escape_len(tail) {
            escaped.push_str(&tail[..len]);
            rest = &tail[len..];
        } else {
            let mut chars = tail.chars();
            let c = chars.next().expect("`find` located a character");
            escaped.push(c);
            if c != '\\' {
                escaped.push(c);
            }
            rest = chars.as_str();
        }
    }
    escaped.push_str(rest);
    Cow::Owned(escaped)
}

/// The byte length of the `\N{...}` name escape opening `text`, `None`
/// when `text` opens on anything else.
fn name_escape_len(text: &str) -> Option<usize> {
    let rest = text.strip_prefix("\\N{")?;
    Some(text.len() - rest.len() + rest.find('}')? + 1)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::{first_value, parse};

    /// The body `src` reads as a template, paired with `"X"` wrapped in
    /// its delimiters.
    fn read(src: &str) -> Option<(String, String)> {
        let source = parse(&format!("X = {src}\n"));
        let expr = first_value(&source)
            .as_string_literal_expr()
            .expect("a string literal");
        let template = Template::read(&source, expr)?;
        Some((template.body.to_owned(), template.wrap("X")))
    }

    #[rstest]
    #[case("plain", "plain")]
    #[case("a {brace} b", "a {{brace}} b")]
    #[case("}closes{", "}}closes{{")]
    #[case("\\N{SNOWMAN}", "\\N{SNOWMAN}")]
    #[case("\\N{SNOWMAN} and {x}", "\\N{SNOWMAN} and {{x}}")]
    #[case("\\\\{x}", "\\\\{{x}}")]
    #[case("\\N{", "\\N{{")]
    fn escape_braces_doubles_every_brace_outside_a_name_escape(
        #[case] text: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(escape_braces(text), expected, "{text}");
    }

    #[rstest]
    fn read_declines_a_template_an_f_string_cannot_carry(
        #[values("\"a\" \"b\"", "u\"text\"", "U\"text\"")] src: &str,
    ) {
        assert!(read(src).is_none(), "{src}");
    }

    #[rstest]
    #[case("\"body\"", "body", "f\"X\"")]
    #[case("'body'", "body", "f'X'")]
    #[case("r\"body\"", "body", "rf\"X\"")]
    #[case("R\"body\"", "body", "rf\"X\"")]
    #[case("\"\"\"body\"\"\"", "body", "f\"\"\"X\"\"\"")]
    fn read_splits_the_body_from_the_delimiters_the_wrap_reuses(
        #[case] src: &str,
        #[case] body: &str,
        #[case] wrapped: &str,
    ) {
        assert_eq!(
            read(src),
            Some((body.to_owned(), wrapped.to_owned())),
            "{src}"
        );
    }
}
