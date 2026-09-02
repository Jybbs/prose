//! Quote unification for the literal parts of a string.

use std::iter;

use ruff_python_ast::{AnyStringFlags, StringFlags, str::Quote};

use crate::primitives::quoting::abuts_triple_closer;

/// The delimiter and literal parts a string settles on once its quotes
/// unify. `parts` pairs positionally with the parts handed to
/// [`requoted`], one entry per literal run.
pub(super) struct Requote {
    pub(super) delimiter: &'static str,
    pub(super) parts: Vec<String>,
}

/// One literal run written both ways, alongside the backslash each
/// delimiter costs across it.
struct Transcription {
    current_body: String,
    current_escapes: usize,
    target_body: String,
    target_escapes: usize,
}

/// The requote a string carrying `parts` settles on under `flags`.
/// `None` covers the raw and triple-quoted declines alone, in that a
/// plain string always reports the quoting it settles on even where
/// that leaves the source untouched.
///
/// A plain string swaps to the opposite delimiter when that drops an
/// escape, and on a tie when the target is `"`. Either way its body
/// comes back carrying exactly the escapes the surviving delimiter
/// needs, so a redundant backslash before the opposite quote is shed.
/// A raw string never gains or loses a backslash, so it swaps only
/// toward `"` and only when every `"` in its parts already carries one.
/// A triple-quoted string swaps to `"""` only when no `"""` run and no
/// trailing `"` would abut the closer.
pub(super) fn requoted(parts: &[&str], flags: AnyStringFlags) -> Option<Requote> {
    requoted_with(parts, flags, true)
}

/// [`requoted`] for a literal whose last part ends ahead of the closer
/// only where `closer_abuts`, as an interpolation following it does.
pub(super) fn requoted_with(
    parts: &[&str],
    flags: AnyStringFlags,
    closer_abuts: bool,
) -> Option<Requote> {
    if flags.is_triple_quoted() {
        return triple_requoted(parts, flags, closer_abuts);
    }
    if flags.prefix().is_raw() {
        return raw_requoted(parts, flags);
    }
    let current = flags.quote_style();
    let target = current.opposite();
    let transcribed: Vec<Transcription> = parts
        .iter()
        .map(|part| transcribe(part, current, target))
        .collect();
    let current_escapes: usize = transcribed.iter().map(|t| t.current_escapes).sum();
    let target_escapes: usize = transcribed.iter().map(|t| t.target_escapes).sum();
    let swap = target_escapes < current_escapes
        || (target_escapes == current_escapes && target.is_double());
    let (delimiter, parts) = if swap {
        (
            delimiter_for(flags, target),
            transcribed.into_iter().map(|t| t.target_body).collect(),
        )
    } else {
        (
            flags.quote_str(),
            transcribed.into_iter().map(|t| t.current_body).collect(),
        )
    };
    Some(Requote { delimiter, parts })
}

/// The `quote_str` a string carrying `flags` reads once its quote style
/// becomes `target`.
fn delimiter_for(flags: AnyStringFlags, target: Quote) -> &'static str {
    AnyStringFlags::new(flags.prefix(), target, flags.triple_quotes()).quote_str()
}

/// Each character of `part` paired with whether a backslash escaped it,
/// the backslash of an escape pair consuming the character after it so
/// a `\\` pair never masks the character beyond. A trailing lone
/// backslash yields itself unescaped.
fn escaped_chars(part: &str) -> impl Iterator<Item = (char, bool)> {
    let mut chars = part.chars();
    iter::from_fn(move || {
        let c = chars.next()?;
        Some(match c {
            '\\' => chars.next().map_or(('\\', false), |next| (next, true)),
            _ => (c, false),
        })
    })
}

/// True when every `target` in `part` already sits behind a backslash,
/// the condition a raw string swaps under.
fn escaped_throughout(part: &str, target: Quote) -> bool {
    escaped_chars(part).all(|(c, escaped)| escaped || c != target.as_char())
}

/// The requote a raw string settles on, `None` when it keeps its
/// delimiters. A raw body carries every backslash into its value, so
/// the swap runs only toward `"`, only when no bare `"` is left to
/// escape, and it leaves the parts byte-for-byte.
fn raw_requoted(parts: &[&str], flags: AnyStringFlags) -> Option<Requote> {
    let target = flags.quote_style().opposite();
    let swappable = target.is_double() && parts.iter().all(|part| escaped_throughout(part, target));
    swappable.then(|| verbatim_requote(parts, flags, target))
}

/// Writes `part` out under both delimiters and counts what each costs.
/// A quote character scores once wherever it appears, whether or not
/// the source spelled it with a backslash, so each body carries only
/// the escapes its own delimiter needs. Every other escape sequence
/// passes through both bodies verbatim.
fn transcribe(part: &str, current: Quote, target: Quote) -> Transcription {
    let mut written = Transcription {
        current_body: String::with_capacity(part.len()),
        current_escapes: 0,
        target_body: String::with_capacity(part.len()),
        target_escapes: 0,
    };
    for (c, escaped) in escaped_chars(part) {
        if c == current.as_char() {
            written.current_escapes += 1;
            written.current_body.push('\\');
            written.current_body.push(c);
            written.target_body.push(c);
        } else if c == target.as_char() {
            written.target_escapes += 1;
            written.current_body.push(c);
            written.target_body.push('\\');
            written.target_body.push(c);
        } else {
            for body in [&mut written.current_body, &mut written.target_body] {
                if escaped {
                    body.push('\\');
                }
                body.push(c);
            }
        }
    }
    written
}

/// The requote a triple-quoted string settles on, `None` when it
/// already reads `"""` or when a `"""` run or a trailing `"` would abut
/// the closer.
fn triple_requoted(parts: &[&str], flags: AnyStringFlags, closer_abuts: bool) -> Option<Requote> {
    let target = Quote::Double;
    let swappable = flags.quote_style() != target && !abuts_triple_closer(parts, closer_abuts);
    swappable.then(|| verbatim_requote(parts, flags, target))
}

/// The requote a string whose body survives byte-for-byte settles on,
/// its parts copied across and only the delimiter moving.
fn verbatim_requote(parts: &[&str], flags: AnyStringFlags, target: Quote) -> Requote {
    Requote {
        delimiter: delimiter_for(flags, target),
        parts: parts.iter().copied().map(str::to_owned).collect(),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_python_ast::{
        str::TripleQuotes,
        str_prefix::{AnyStringPrefix, StringLiteralPrefix},
    };

    use super::*;

    fn flags(quote: Quote, triple: TripleQuotes, prefix: AnyStringPrefix) -> AnyStringFlags {
        AnyStringFlags::new(prefix, quote, triple)
    }

    fn plain(quote: Quote) -> AnyStringFlags {
        flags(
            quote,
            TripleQuotes::No,
            AnyStringPrefix::Regular(StringLiteralPrefix::Empty),
        )
    }

    fn raw(quote: Quote) -> AnyStringFlags {
        flags(
            quote,
            TripleQuotes::No,
            AnyStringPrefix::Regular(StringLiteralPrefix::Raw { uppercase: false }),
        )
    }

    fn triple(quote: Quote) -> AnyStringFlags {
        flags(
            quote,
            TripleQuotes::Yes,
            AnyStringPrefix::Regular(StringLiteralPrefix::Empty),
        )
    }

    #[rstest]
    #[case("ab", &[('a', false), ('b', false)])]
    #[case(r"a\'b", &[('a', false), ('\'', true), ('b', false)])]
    #[case(r"a\\'", &[('a', false), ('\\', true), ('\'', false)])]
    #[case("a\\", &[('a', false), ('\\', false)])]
    fn escaped_chars_pairs_each_character_with_the_backslash_that_reached_it(
        #[case] part: &str,
        #[case] expected: &[(char, bool)],
    ) {
        assert_eq!(escaped_chars(part).collect::<Vec<_>>(), expected);
    }

    #[rstest]
    fn requoted_holds_a_double_quoted_body_that_would_not_shed_an_escape(
        #[values("plain", "it's", "")] body: &str,
    ) {
        let requote = requoted(&[body], plain(Quote::Double)).expect("reports its own quoting");
        assert_eq!(requote.delimiter, "\"");
        assert_eq!(requote.parts, [body]);
    }

    #[rstest]
    #[case(r#"say "hi""#, r#"say "hi""#)]
    #[case(r#"it\'s "x""#, r#"it\'s "x""#)]
    fn requoted_holds_a_single_quoted_body_that_would_gain_escapes(
        #[case] body: &str,
        #[case] expected: &str,
    ) {
        let requote = requoted(&[body], plain(Quote::Single)).expect("reports its own quoting");
        assert_eq!(requote.delimiter, "'");
        assert_eq!(requote.parts, [expected]);
    }

    #[test]
    fn requoted_holds_a_triple_quoted_body_already_double() {
        assert!(requoted(&["plain"], triple(Quote::Double)).is_none());
    }

    #[rstest]
    fn requoted_holds_a_triple_quoted_body_that_would_abut_the_closer(
        #[values(r#"say "hi""#, r#"ends in quote""#, r#"has """ run"#)] body: &str,
    ) {
        assert!(requoted(&[body], triple(Quote::Single)).is_none());
    }

    #[rstest]
    #[case(Quote::Single, r#"  File \"{x}\", line"#, "'", r#"  File "{x}", line"#)]
    #[case(Quote::Double, r"needless \' escape", "\"", r"needless ' escape")]
    fn requoted_sheds_a_redundant_escape_while_holding_the_delimiter(
        #[case] quote: Quote,
        #[case] body: &str,
        #[case] delimiter: &str,
        #[case] expected: &str,
    ) {
        let requote = requoted(&[body], plain(quote)).expect("reports its own quoting");
        assert_eq!(requote.delimiter, delimiter);
        assert_eq!(requote.parts, [expected]);
    }

    #[test]
    fn requoted_swaps_a_double_quoted_body_that_sheds_an_escape() {
        let requote = requoted(&[r#"already \" escaped"#], plain(Quote::Double)).expect("swaps");
        assert_eq!(requote.delimiter, "'");
        assert_eq!(requote.parts, [r#"already " escaped"#]);
    }

    #[rstest]
    #[case(r"raw\d", true)]
    #[case(r#"raw \"dq\" \d"#, true)]
    #[case(r#"raw "dq" \d"#, false)]
    fn requoted_swaps_a_raw_body_only_when_no_bare_target_quote_remains(
        #[case] body: &str,
        #[case] swaps: bool,
    ) {
        let requote = requoted(&[body], raw(Quote::Single));
        assert_eq!(requote.is_some(), swaps);
        if let Some(requote) = requote {
            assert_eq!(requote.parts, [body], "a raw body is never rewritten");
        }
    }

    #[rstest]
    #[case("plain", "\"", "plain")]
    #[case("", "\"", "")]
    #[case(r"it\'s", "\"", "it's")]
    #[case(r#"both \' and ""#, "\"", r#"both ' and \""#)]
    #[case(r"tab\there", "\"", r"tab\there")]
    #[case(r"backslash\\", "\"", r"backslash\\")]
    fn requoted_swaps_a_single_quoted_body(
        #[case] body: &str,
        #[case] delimiter: &str,
        #[case] expected: &str,
    ) {
        let requote = requoted(&[body], plain(Quote::Single)).expect("swaps");
        assert_eq!(requote.delimiter, delimiter);
        assert_eq!(requote.parts, [expected]);
    }

    #[test]
    fn requoted_swaps_a_triple_quoted_body_verbatim() {
        let requote = requoted(&[r"a \' b"], triple(Quote::Single)).expect("swaps");
        assert_eq!(requote.delimiter, "\"\"\"");
        assert_eq!(requote.parts, [r"a \' b"]);
    }

    #[test]
    fn requoted_weighs_every_part_of_a_multi_part_body_together() {
        let held = requoted(&[r"it\'s ", r#"a "quoted" word"#], plain(Quote::Single))
            .expect("reports its own quoting");
        assert_eq!(
            held.delimiter, "'",
            "two added escapes outweigh the one dropped"
        );

        let swapped = requoted(&[r"it\'s ", r"a \'quoted\' word"], plain(Quote::Single))
            .expect("three escapes drop and none are added");
        assert_eq!(swapped.delimiter, "\"");
        assert_eq!(swapped.parts, ["it's ", "a 'quoted' word"]);
    }
}
