//! Normalizes blank-line spacing between adjacent statements at module,
//! class, and function scopes. The walker pairs each statement with its
//! predecessor and emits edits to bring the gap to the canonical count
//! returned by `canonical_blanks`. Own-line comments between adjacent
//! statements carry 1 blank line above the comment block, 0 blank lines
//! below a description block, and 1 blank line below a banner block.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt,
    statement_visitor::{StatementVisitor, walk_stmt},
};
use ruff_python_trivia::{lines_after, lines_before};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        edit::singleton_groups,
        scope::{BodyScope, scoped_body},
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod comment_block;
mod offsets;
mod policy;

use comment_block::{is_banner_block, leading_block_of};
use offsets::{header_signature_end, whitespace_start_before};
use policy::canonical_blanks;

pub(crate) struct BlankLines {
    first_party: Vec<String>,
}

impl BlankLines {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            first_party: config.first_party(),
        }
    }
}

impl Rule for BlankLines {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let body = &source.ast().body;
        let mut walker = Walker {
            edits: Vec::new(),
            first_party: &self.first_party,
            source,
        };
        walker.pair_siblings(body, BodyScope::Module);
        walker.visit_body(body);
        singleton_groups(walker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    edits: Vec<Edit>,
    first_party: &'a [String],
    source: &'a Source,
}

impl Walker<'_> {
    /// Places `target_newlines` line breaks immediately above
    /// `line_start`. Emits a replacement edit when the actual count
    /// differs. Preserves any indent that sits on `line_start`'s line.
    fn normalize_above(&mut self, line_start: TextSize, target_newlines: u32) {
        let text = self.source.text();
        if lines_before(line_start, text) == target_newlines {
            return;
        }
        let span_start = whitespace_start_before(text, line_start);
        let replacement = self.source.newline_str().repeat(target_newlines as usize);
        self.edits.push(Edit::range_replacement(
            replacement,
            TextRange::new(span_start, line_start),
        ));
    }

    /// Places `target_newlines` line breaks between `block_end` and
    /// `curr_line_start`. Emits a replacement edit when the actual
    /// count differs.
    fn normalize_below_block(
        &mut self,
        block_end: TextSize,
        curr_line_start: TextSize,
        target_newlines: u32,
    ) {
        if lines_after(block_end, self.source.text()) == target_newlines {
            return;
        }
        self.edits.push(Edit::range_replacement(
            self.source.newline_str().repeat(target_newlines as usize),
            TextRange::new(block_end, curr_line_start),
        ));
    }

    fn pair_in_scope(&mut self, header: &Stmt, body: &[Stmt], scope: BodyScope) {
        if let Some(first) = body.first() {
            let prev_end = header_signature_end(self.source, first.start());
            // A single-line suite opens its body on the header line, leaving no
            // own-line gap above it to normalize. Pairing it would collide with
            // the sibling pair over the gap above the header's own line.
            if !self.source.same_line(prev_end, first.start()) {
                self.pair_with_end(header, prev_end, first, scope);
            }
        }
        self.pair_siblings(body, scope);
    }

    fn pair_siblings(&mut self, body: &[Stmt], scope: BodyScope) {
        for (prev, curr) in body.iter().zip(body.iter().skip(1)) {
            self.pair_with_end(prev, prev.end(), curr, scope);
        }
    }

    fn pair_with_end(&mut self, prev: &Stmt, prev_end: TextSize, curr: &Stmt, scope: BodyScope) {
        let Some(canonical) = canonical_blanks(prev, curr, scope, self.first_party) else {
            return;
        };
        let block = leading_block_of(self.source, prev_end, curr);
        let curr_line_start = self.source.text().line_start(curr.start());
        let above_line_start = block.map_or(curr_line_start, TextRange::start);
        self.normalize_above(above_line_start, canonical + 1);
        if let Some(b) = block {
            let below_target = 1 + u32::from(is_banner_block(self.source, b));
            self.normalize_below_block(b.end(), curr_line_start, below_target);
        }
    }
}

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Some((body, scope)) = scoped_body(stmt) {
            self.pair_in_scope(stmt, body, scope);
        }
        walk_stmt(self, stmt);
    }
}
