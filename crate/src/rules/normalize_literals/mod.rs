//! Canonical literal spelling, gathered into one pass over the token
//! stream. `unify-quotes` settles a string on `"`, falling to `'` only
//! where that drops an escape, and leaves the body carrying just the
//! escapes its surviving delimiter needs. `unify-prefixes` lowercases a
//! string prefix and drops the no-op `u`. `unify-numerics` uppercases hex
//! digits while lowercasing the radix marker, the exponent, and the `j`
//! suffix. The quote facet passes over the docstring slot, whose frame
//! `frame-docstrings` owns, and over any literal inside a replacement
//! field, whose quotes the enclosing string constrains before Python
//! 3.12.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyStringFlags, StringFlags,
    token::{Token, TokenKind},
};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        docstring::docstring_slots, edit::narrowed_replacement,
        tokens::is_interpolated_string_start,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod numeric;
mod prefix;
mod quote;

use self::{numeric::canonical_number, prefix::canonical_prefix, quote::requoted};

pub(crate) struct NormalizeLiterals {
    unify_numerics: bool,
    unify_prefixes: bool,
    unify_quotes: bool,
}

impl NormalizeLiterals {
    pub(crate) const MESSAGE: &'static str =
        "canonicalize literal quote, prefix, and numeric spelling";

    pub(crate) fn from_config(config: &Config) -> Self {
        let rule = &config.rules.normalize_literals;
        Self {
            unify_numerics: rule.unify_numerics,
            unify_prefixes: rule.unify_prefixes,
            unify_quotes: rule.unify_quotes,
        }
    }
}

impl Rule for NormalizeLiterals {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut normalizer = Normalizer {
            docstrings: if self.unify_quotes {
                docstring_slots(&source.ast().body)
            } else {
                Vec::new()
            },
            groups: Vec::new(),
            open: Vec::new(),
            rule: self,
            source,
        };
        normalizer.walk();
        normalizer.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// An f-string or t-string whose opener has been read and whose closer
/// has not, gathering the literal runs between the two.
struct Frame {
    flags: AnyStringFlags,
    middles: Vec<TextRange>,
    opener: TextRange,
}

struct Normalizer<'a> {
    docstrings: Vec<TextRange>,
    groups: Vec<Vec<Edit>>,
    open: Vec<Frame>,
    rule: &'a NormalizeLiterals,
    source: &'a Source,
}

impl<'a> Normalizer<'a> {
    /// Emits the edits closing the interpolated string `frame` opened,
    /// its opener carrying the prefix and both delimiters moving
    /// together with every literal run between them.
    fn close(&mut self, closer: TextRange) {
        let frame = self
            .open
            .pop()
            .expect("an interpolated-string closer follows its opener");
        let middles: Vec<&str> = frame
            .middles
            .iter()
            .map(|&range| self.source.slice(range))
            .collect();
        let requote = self
            .interpolations_clear(&frame, closer)
            .then(|| requoted(&middles, frame.flags))
            .flatten();
        let prefix = self.prefix_text(frame.opener, frame.flags);
        let delimiter = requote
            .as_ref()
            .map_or_else(|| frame.flags.quote_str(), |requote| requote.delimiter);
        let mut edits = Vec::new();
        edits.extend(narrowed_replacement(
            self.source,
            frame.opener,
            format!("{prefix}{delimiter}"),
        ));
        if let Some(requote) = requote {
            for (&range, text) in frame.middles.iter().zip(requote.parts) {
                edits.extend(narrowed_replacement(self.source, range, text));
            }
            edits.extend(narrowed_replacement(
                self.source,
                closer,
                delimiter.to_owned(),
            ));
        }
        if !edits.is_empty() {
            self.groups.push(edits);
        }
    }

    /// True when the quote facet reaches `frame` and no replacement
    /// field between its opener and `closer` carries the quote the swap
    /// would move to. The literal runs and the replacement fields
    /// partition the span between the delimiters, so a target quote the
    /// runs do not account for sits in a field.
    fn interpolations_clear(&self, frame: &Frame, closer: TextRange) -> bool {
        if !self.quotes_reach(frame.opener) {
            return false;
        }
        let target = frame.flags.quote_style().opposite().as_char();
        let inner = TextRange::new(frame.opener.end(), closer.start());
        let in_runs: usize = frame
            .middles
            .iter()
            .map(|&range| self.source.slice(range).matches(target).count())
            .sum();
        self.source.slice(inner).matches(target).count() == in_runs
    }

    /// Emits the edit respelling the numeric literal at `range`.
    fn number(&mut self, range: TextRange) {
        if !self.rule.unify_numerics {
            return;
        }
        if let Cow::Owned(text) = canonical_number(self.source.slice(range)) {
            self.push(narrowed_replacement(self.source, range, text));
        }
    }

    /// The prefix a literal opening at `range` under `flags` carries
    /// once the prefix facet has run, or its source text when that
    /// facet is off.
    fn prefix_text(&self, range: TextRange, flags: AnyStringFlags) -> &'a str {
        if self.rule.unify_prefixes {
            canonical_prefix(flags.prefix())
        } else {
            self.source
                .slice(TextRange::at(range.start(), flags.prefix().text_len()))
        }
    }

    /// Files `edit` as a fix group of its own, the shape a literal
    /// whose respelling stands independent of every other one takes.
    fn push(&mut self, edit: Option<Edit>) {
        self.groups.extend(edit.map(|edit| vec![edit]));
    }

    /// True when the quote facet reaches a literal opening at `range`,
    /// which passes over a docstring slot and anything nested inside a
    /// replacement field.
    fn quotes_reach(&self, range: TextRange) -> bool {
        self.rule.unify_quotes
            && self.open.is_empty()
            && !self
                .docstrings
                .iter()
                .any(|slot| slot.contains_range(range))
    }

    /// Emits the edit respelling the plain string or bytes literal
    /// `token` holds, prefix and both delimiters landing as one
    /// replacement over the whole token.
    fn string(&mut self, token: &Token) {
        let flags = token.unwrap_string_flags();
        let range = token.range();
        let body = TextRange::new(
            range.start() + flags.opener_len(),
            range.end() - flags.closer_len(),
        );
        let source_body = self.source.slice(body);
        let requote = self
            .quotes_reach(range)
            .then(|| requoted(&[source_body], flags))
            .flatten();
        let prefix = self.prefix_text(range, flags);
        let (delimiter, text) = requote.as_ref().map_or_else(
            || (flags.quote_str(), source_body),
            |requote| (requote.delimiter, requote.parts[0].as_str()),
        );
        self.push(narrowed_replacement(
            self.source,
            range,
            format!("{prefix}{delimiter}{text}{delimiter}"),
        ));
    }

    /// Reads the token stream once, routing each numeric literal and
    /// each plain string to its own respelling and pairing every
    /// interpolated string's opener with the closer that ends it.
    fn walk(&mut self) {
        let source = self.source;
        for token in source.tokens() {
            let kind = token.kind();
            match kind {
                _ if is_interpolated_string_start(kind) => self.open.push(Frame {
                    flags: token.unwrap_string_flags(),
                    middles: Vec::new(),
                    opener: token.range(),
                }),
                _ if kind.is_interpolated_string_end() => self.close(token.range()),
                TokenKind::Complex | TokenKind::Float | TokenKind::Int => {
                    self.number(token.range());
                }
                TokenKind::FStringMiddle | TokenKind::TStringMiddle => self
                    .open
                    .last_mut()
                    .expect("a literal run sits inside an open interpolated string")
                    .middles
                    .push(token.range()),
                TokenKind::String => self.string(token),
                _ => {}
            }
        }
    }
}
