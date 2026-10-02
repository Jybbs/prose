//! The wrap options every emission shares and the line-continuation
//! splice a non-raw body resolves before wrapping.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_text_size::TextSize;
use textwrap::{Options, WordSeparator, WordSplitter, core::Word};

use crate::primitives::docstring::opens_structure;

/// True where `text` closes on an odd run of backslashes, which in a
/// non-raw body escapes the newline a row ending there carries.
pub(super) fn ends_on_continuation(text: &str) -> bool {
    !(text.len() - text.trim_end_matches('\\').len()).is_multiple_of(2)
}

/// Splits `content` on `newline`, merging a continued line with the one
/// below it when neither side of the dropped backslash has whitespace.
/// Every other line passes through as written, leaving a continuation
/// inside a passthrough region byte-identical. Each line is paired with
/// the byte offset of its first physical line within `content`.
pub(super) fn spliced_continuations<'a>(
    content: &'a str,
    newline: &str,
    raw: bool,
) -> Vec<(TextSize, Cow<'a, str>)> {
    let mut lines: Vec<(TextSize, Cow<'a, str>)> = Vec::new();
    let mut physical = content.split(newline).peekable();
    let mut splicing = false;
    let mut offset = TextSize::default();
    while let Some(line) = physical.next() {
        let start = offset;
        offset += TextSize::of(line) + TextSize::of(newline);
        let head = without_continuation(line, raw);
        let tight = head.len() < line.len()
            && !head.ends_with(char::is_whitespace)
            && physical
                .peek()
                .is_some_and(|next| !next.starts_with(char::is_whitespace));
        let text = if tight { head } else { line };
        match lines.last_mut().filter(|_| splicing) {
            Some((_, last)) => last.to_mut().push_str(text),
            None => lines.push((start, Cow::Borrowed(text))),
        }
        splicing = tight;
    }
    lines
}

/// Drops the trailing backslash of a line continuation, leaving the join
/// to the paragraph collapse, which reads the whitespace on either side
/// of it as the separator. An odd run of trailing backslashes closes on
/// a continuation and an even run closes on an escaped backslash, and a
/// raw docstring holds no continuations at all.
pub(super) fn without_continuation(line: &str, raw: bool) -> &str {
    if raw || !ends_on_continuation(line) {
        return line;
    }
    &line[..line.len() - 1]
}

/// The wrap options every emission shares. The custom separator and
/// `NoHyphenation` keep a slash- or hyphen-bearing token atomic,
/// leaving an over-budget URL or path to overflow unsplit, and the
/// separator reads `raw` for whether a backslash run ending a row
/// continues it.
pub(super) fn wrap_options<'o>(
    width: usize,
    initial: &'o str,
    subsequent: &'o str,
    raw: bool,
) -> Options<'o> {
    let separator: fn(&str) -> Box<dyn Iterator<Item = Word<'_>> + '_> = if raw {
        |line| words(line, true)
    } else {
        |line| words(line, false)
    };
    Options::new(width)
        .break_words(false)
        .initial_indent(initial)
        .subsequent_indent(subsequent)
        .word_separator(WordSeparator::Custom(separator))
        .word_splitter(WordSplitter::NoHyphenation)
}

/// Splits `line` on ASCII spaces, dropping every break opportunity
/// whose remainder would open a verbatim structure, a list marker, a
/// section heading, or an entry head, folding that break into the word
/// before it. Outside a `raw` body a break after a word closing on an
/// odd run of backslashes folds the same way.
fn words(line: &str, raw: bool) -> Box<dyn Iterator<Item = Word<'_>> + '_> {
    let mut starts = Vec::new();
    let mut cursor = 0;
    let mut continues = false;
    for word in WordSeparator::AsciiSpace.find_words(line) {
        if starts.is_empty() || !(continues || opens_structure(&line[cursor..])) {
            starts.push(cursor);
        }
        continues = !raw && ends_on_continuation(word.word);
        cursor += word.word.len() + word.whitespace.len();
    }
    starts.push(line.len());
    Box::new(
        starts
            .into_iter()
            .tuple_windows()
            .map(|(start, end)| Word::from(&line[start..end])),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{ends_on_continuation, spliced_continuations, without_continuation, words};

    #[rstest]
    #[case::plain_prose("plain prose", false)]
    #[case::one_backslash("continues \\", true)]
    #[case::two_backslashes("escaped \\\\", false)]
    #[case::three_backslashes("escaped then continues \\\\\\", true)]
    fn ends_on_continuation_reads_an_odd_run_of_backslashes(
        #[case] text: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(ends_on_continuation(text), expected);
    }

    #[rstest]
    #[case("see https://host/\\\npath.html", false, &["see https://host/path.html"])]
    #[case("trailing run \\\n    indented", false, &["trailing run \\", "    indented"])]
    #[case("spaced out \\\nflush", false, &["spaced out \\", "flush"])]
    #[case("raw https://host/\\\npath.html", true, &["raw https://host/\\", "path.html"])]
    fn spliced_continuations_merges_only_a_join_carrying_no_whitespace(
        #[case] content: &str,
        #[case] raw: bool,
        #[case] expected: &[&str],
    ) {
        let lines: Vec<_> = spliced_continuations(content, "\n", raw)
            .into_iter()
            .map(|(_, line)| line)
            .collect();
        assert_eq!(lines, expected);
    }

    #[rstest]
    #[case("plain prose", false, "plain prose")]
    #[case("escaped \\\\", false, "escaped \\\\")]
    #[case("continues \\", false, "continues ")]
    #[case("literal \\", true, "literal \\")]
    fn without_continuation_drops_only_an_odd_run_in_a_non_raw_body(
        #[case] line: &str,
        #[case] raw: bool,
        #[case] expected: &str,
    ) {
        assert_eq!(without_continuation(line, raw), expected);
    }

    #[rstest]
    #[case::non_raw_joins_the_word_after_a_backslash(false, &["except", "\\ (", ")>"])]
    #[case::raw_keeps_every_break(true, &["except", "\\", "(", ")>"])]
    fn words_join_a_word_after_an_odd_backslash_run_outside_a_raw_body(
        #[case] raw: bool,
        #[case] expected: &[&str],
    ) {
        let found: Vec<&str> = words("except \\ ( )>", raw).map(|word| word.word).collect();
        assert_eq!(found, expected);
    }
}
