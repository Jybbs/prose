//! The buffered prose run a wrap emits as one paragraph, its head kept
//! verbatim and its description rewrapped from the column that head
//! settles to.

use std::borrow::Cow;

use itertools::Itertools;

use super::{Region, Walker, wrapping::wrap_options};

#[derive(Default)]
pub(super) struct Paragraph<'a> {
    pub(super) head: &'a str,
    pub(super) head_slack: usize,
    pub(super) initial_indent: &'a str,
    pub(super) lines: Vec<&'a str>,
    pub(super) subsequent_indent: Cow<'a, str>,
}

impl Walker<'_> {
    pub(super) fn flush_paragraph(&mut self) {
        if !self.paragraph.lines.is_empty() {
            let Paragraph {
                head,
                head_slack,
                initial_indent,
                lines,
                subsequent_indent,
            } = std::mem::take(&mut self.paragraph);
            // The head is a fixed prefix rather than wrappable text, so
            // it joins the initial indent, which `textwrap` never breaks
            // inside and never emits a row without a word after.
            let opening = [initial_indent, head].concat();
            let text = collapsed(lines);
            if head_slack == 0 {
                self.emit_wrapped(
                    &opening,
                    &subsequent_indent,
                    &text,
                    self.rule.description_width,
                );
            } else {
                // The rows break at the width the padding rule settles
                // the head to, the head itself emitted as written with
                // its padding left to that rule's edit.
                let measured = " ".repeat(opening.chars().count().saturating_sub(head_slack));
                let mut pieces = textwrap::wrap(
                    &text,
                    wrap_options(self.rule.description_width, &measured, &subsequent_indent),
                )
                .into_iter();
                if let Some(first) = pieces.next() {
                    self.emit_verbatim(&[opening.as_str(), &first[measured.len()..]].concat());
                }
                for piece in pieces {
                    self.emit_verbatim(&piece);
                }
            }
        }
        if self.region == Region::SectionEntry {
            self.region = Region::Section;
        }
    }

    fn emit_wrapped(&mut self, initial: &str, subsequent: &str, text: &str, width: usize) {
        for piece in textwrap::wrap(text, wrap_options(width, initial, subsequent)) {
            self.emit_verbatim(&piece);
        }
    }
}

/// Joins `lines` into one prose run, collapsing every whitespace run
/// between words to a single space.
pub(super) fn collapsed<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    lines.into_iter().flat_map(str::split_whitespace).join(" ")
}

#[cfg(test)]
mod tests {
    use super::collapsed;

    #[test]
    fn collapsed_joins_lines_and_squeezes_interior_runs() {
        assert_eq!(collapsed(["one  two", "three\tfour"]), "one two three four");
    }
}
