//! The sides of a pair that run into an identifier character, deciding
//! where a removal leaves a space behind.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_text_size::TextRange;

use crate::{primitives::edit::joins_an_identifier, source::Source};

/// The sides of a pair that touch an identifier character, the keyword
/// written flush against its paren, where the pair's removal leaves a
/// single space rather than nothing.
pub(super) struct Flush {
    after: bool,
    before: bool,
}

impl Flush {
    pub(super) fn of(source: &Source, pair: TextRange) -> Self {
        let text = source.text();
        let touches = |c: Option<char>| c.is_some_and(joins_an_identifier);
        Self {
            after: touches(text[pair.end().to_usize()..].chars().next()),
            before: touches(text[..pair.start().to_usize()].chars().next_back()),
        }
    }

    /// The edit removing `span`, one of the pair's parens with the
    /// whitespace it takes along, leaving a space where `flush` says
    /// the side touches an identifier character.
    fn removal(flush: bool, span: TextRange) -> Edit {
        if flush {
            Edit::range_replacement(" ".to_owned(), span)
        } else {
            Edit::range_deletion(span)
        }
    }

    /// The two edits taking a pair's parentheses out, `open` covering
    /// the opening side and `close` the closing one, each leaving a
    /// space where that side runs into an identifier character.
    pub(super) fn flanking(&self, open: TextRange, close: TextRange) -> [Edit; 2] {
        [
            Self::removal(self.before, open),
            Self::removal(self.after, close),
        ]
    }

    /// `text` with the space each flush side keeps, the form the splice
    /// probe and the emitted edits share.
    pub(super) fn padded<'t>(&self, text: &'t str) -> Cow<'t, str> {
        match (self.before, self.after) {
            (false, false) => Cow::Borrowed(text),
            (before, after) => {
                let lead = if before { " " } else { "" };
                let trail = if after { " " } else { "" };
                Cow::Owned(format!("{lead}{text}{trail}"))
            }
        }
    }

    /// How many columns the pair's removal keeps as spaces.
    pub(super) fn spaces(&self) -> usize {
        usize::from(self.before) + usize::from(self.after)
    }
}
