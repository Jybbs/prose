//! The wrap options every emission shares and the line-continuation
//! splice a non-raw body resolves before wrapping.

use std::borrow::Cow;

use itertools::Itertools;
use ruff_text_size::TextSize;
use textwrap::{Options, WordSeparator, WordSplitter, core::Word};

use crate::primitives::docstring::opens_structure;

/// Splits `line` on ASCII spaces, dropping every break opportunity
/// whose remainder opens a verbatim structure. A row head reading as a
/// list marker, a section heading, an entry head, or any other
/// structure would be parsed as that structure on the next pass, so the
/// break folds back into the word before it and the run stays on one
/// row.
fn prose_words(line: &str) -> Box<dyn Iterator<Item = Word<'_>> + '_> {
    let mut starts = Vec::new();
    let mut cursor = 0;
    for word in WordSeparator::AsciiSpace.find_words(line) {
        if starts.is_empty() || !opens_structure(&line[cursor..]) {
            starts.push(cursor);
        }
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

/// Splits `content` on `newline`, merging a continued line with the one
/// below it when neither side of the dropped backslash carries
/// whitespace, the one join the paragraph collapse cannot reproduce.
/// Every other line passes through split as written, leaving a
/// continuation inside a passthrough region byte-identical. Each line
/// is paired with the byte offset of its first physical line within
/// `content`.
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
    let backslashes = line.len() - line.trim_end_matches('\\').len();
    if raw || backslashes.is_multiple_of(2) {
        return line;
    }
    &line[..line.len() - 1]
}

/// The wrap options every emission shares. The custom separator keeps a
/// slash- or hyphen-bearing token atomic alongside `NoHyphenation`, so
/// an over-budget URL or path overflows instead of splitting.
pub(super) fn wrap_options<'o>(width: usize, initial: &'o str, subsequent: &'o str) -> Options<'o> {
    Options::new(width)
        .break_words(false)
        .initial_indent(initial)
        .subsequent_indent(subsequent)
        .word_separator(WordSeparator::Custom(prose_words))
        .word_splitter(WordSplitter::NoHyphenation)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{spliced_continuations, without_continuation};

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
}
