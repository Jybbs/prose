//! The per-line walk a docstring body takes, dispatching each scanned
//! line to the region that owns it.

use std::borrow::Cow;

use ruff_text_size::{TextRange, TextSize};
use textwrap::WrapAlgorithm;

use super::{
    Region, Walker,
    paragraph::collapsed,
    wrapping::{without_continuation, wrap_options},
};
use crate::primitives::{
    docstring::{LineScan, ScannedLine, section_heading, sibling_entry_head, typed_entry_head},
    padding,
};

impl<'a> Walker<'a> {
    fn buffer_description(&mut self, indent: &'a str, text: &'a str) {
        if self.paragraph.lines.is_empty() {
            self.paragraph.initial_indent = indent;
            self.paragraph.subsequent_indent = Cow::Borrowed(indent);
        }
        self.paragraph.lines.push(text);
    }

    fn flush_verbatim(&mut self, line: &str) {
        self.flush_paragraph();
        self.emit_verbatim(line);
    }

    /// True when `trimmed` continues the open entry's description,
    /// sitting from the section body indent onward and opening no
    /// sibling entry there.
    fn is_entry_continuation(&self, indent_chars: usize, trimmed: &str) -> bool {
        let section_body = self.scanner.section_body_indent_chars();
        indent_chars >= section_body
            && sibling_entry_head(indent_chars, section_body, trimmed).is_none()
    }

    fn start_entry(
        &mut self,
        indent_str: &'a str,
        indent_chars: usize,
        text: &'a str,
        desc_start: usize,
        line_offset: TextSize,
    ) {
        let (head, description) = text.split_at(desc_start);
        // The hang column reads the head at the width the padding rule
        // settles it to.
        let start = self.content_start + line_offset + TextSize::of(indent_str);
        let range = TextRange::at(start, TextSize::of(head));
        let slack = padding::slack(self.source, self.padding, range)
            .max(0)
            .cast_unsigned();
        self.paragraph.head = head;
        self.paragraph.head_slack = slack;
        self.paragraph.initial_indent = indent_str;
        self.paragraph.subsequent_indent = " "
            .repeat((indent_chars + head.chars().count()).saturating_sub(slack))
            .into();
        self.paragraph.lines.push(description);
        self.region = Region::SectionEntry;
    }

    pub(super) fn consume(&mut self, offset: TextSize, line: &'a str) {
        let ScannedLine {
            indent,
            indent_chars,
            scan,
            trimmed,
        } = self.scanner.scan_line(line);

        match scan {
            LineScan::Fence | LineScan::ListMarker | LineScan::VerbatimOpen => {
                self.flush_verbatim(line);
                return;
            }
            LineScan::InFence | LineScan::ListContinuation | LineScan::Verbatim => {
                self.emit_verbatim(line);
                return;
            }
            LineScan::Blank => {
                self.flush_paragraph();
                self.out.push_str(self.newline);
                return;
            }
            LineScan::Body => {}
        }

        let body_indent = self.scanner.body_indent_chars();
        if indent_chars == body_indent && section_heading(trimmed).is_some() {
            self.flush_verbatim(line);
            self.region = Region::Section;
            return;
        }

        let text = without_continuation(trimmed, self.raw).trim_end();
        if self.region == Region::SectionEntry {
            if self.is_entry_continuation(indent_chars, text) {
                self.paragraph.lines.push(text);
                return;
            }
            self.flush_paragraph();
        }

        let prose_indent = match self.region {
            Region::Description => body_indent,
            Region::Section => self.scanner.section_body_indent_chars(),
            Region::SectionEntry => unreachable!("entries handled above"),
        };
        if indent_chars > prose_indent {
            self.flush_verbatim(line);
            return;
        }

        if self.region == Region::Section && indent_chars < prose_indent {
            self.region = Region::Description;
        }

        match self.region {
            Region::Description if self.paragraph.lines.is_empty() && typed_entry_head(text) => {
                self.flush_verbatim(line);
            }
            Region::Description => self.buffer_description(indent, text),
            Region::Section => {
                if let Some(head) = sibling_entry_head(indent_chars, prose_indent, text) {
                    self.start_entry(indent, indent_chars, text, head.desc_start, offset);
                    return;
                }
                // Section prose wraps one line at a time with no
                // paragraph rejoin, so it takes the first-fit algorithm,
                // whose maximal lines re-wrap to themselves.
                let opts = wrap_options(self.rule.section_width, indent, indent)
                    .wrap_algorithm(WrapAlgorithm::FirstFit);
                for piece in textwrap::wrap(&collapsed([text]), opts) {
                    self.emit_verbatim(&piece);
                }
            }
            Region::SectionEntry => unreachable!("entries handled above"),
        }
    }

    pub(super) fn emit_verbatim(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push_str(self.newline);
    }
}
