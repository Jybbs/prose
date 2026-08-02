//! Breaks a fluent method chain across lines under two triggers. The
//! count trigger fires on a chain carrying more than `max_links` links,
//! and the length trigger on one whose joined single-line form crosses
//! `code_line_length` from the column it lands at. The broken chain sits
//! inside a parenthesis pair, reusing one the source already carries,
//! with its head holding the receiver and the first link and every later
//! link hanging beneath the head's own dot. A receiver wider than
//! `max_shift` takes the full split instead, standing alone with every
//! link flush beneath it. A `.name` access that is not itself called
//! shares the row of the link below it. Neither trigger reaches a chain
//! inside an f-string or t-string replacement field, one spanning a
//! comment, or one whose segments already carry a line break.
//!
//! `spine` divides a chain into its receiver and links, and `render`
//! builds the text that replaces it.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor as AstVisitor, walk_arguments, walk_expr, walk_stmt},
};
use ruff_text_size::TextSize;

use crate::{
    config::{Config, MaxShift},
    primitives::{
        aligner,
        edit::{insert_edit, narrowed_replacement, singleton_groups},
        reserve::{reserved_columns, settled_column},
    },
    rule::{Rule, RuleId},
    rules::align_equals::AlignEquals,
    source::Source,
};

mod render;
mod spine;

use spine::Chain;

pub(crate) struct ChainLayout {
    align_equals: Option<aligner::Settings>,
    code_line_length: usize,
    max_links: Option<usize>,
    max_shift: MaxShift,
}

impl ChainLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.chain_layout;
        Self {
            align_equals: AlignEquals::reserve_settings(config),
            code_line_length: config.code_width(),
            max_links: rules.max_links.cap(),
            max_shift: rules.max_shift,
        }
    }
}

impl Rule for ChainLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let reservations = reserved_columns(source, self.align_equals, AlignEquals::SLUG);
        let mut breaker = Breaker {
            cap: self.max_links,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_shift: self.max_shift,
            parent: AnyNodeRef::from(source.ast()),
            reservations: &reservations,
            source,
        };
        breaker.visit_body(&source.ast().body);
        singleton_groups(breaker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Walks a module emitting the break edit each over-long or over-count
/// chain needs. `parent` is the node enclosing the expression under
/// visit, the node a grouping-pair recovery resolves against.
struct Breaker<'a> {
    cap: Option<usize>,
    code_line_length: usize,
    edits: Vec<Edit>,
    max_shift: MaxShift,
    parent: AnyNodeRef<'a>,
    reservations: &'a HashMap<TextSize, usize>,
    source: &'a Source,
}

impl<'a> Breaker<'a> {
    /// The edit breaking `chain` across lines, or `None` where neither
    /// trigger fires, a comment or a line-spanning segment holds the
    /// shape, or the chain already reads as the break's own output.
    fn break_chain(&self, expr: &'a Expr, chain: &Chain<'a>) -> Option<Edit> {
        let range = self.source.paren_aware_range(expr.into(), self.parent);
        if self.source.intersects_comment(range)
            || chain.spans_lines(self.source)
            || !self.trips(chain, range.start())
        {
            return None;
        }
        let indent = self.source.line_indent_width(range.start());
        let text = render::broken(self.source, chain, indent, self.hang(chain));
        narrowed_replacement(self.source, range, text)
    }

    /// The columns each link's dot hangs past the head's indent, `None`
    /// where the receiver runs wider than `max_shift` allows and the
    /// chain takes the full split.
    fn hang(&self, chain: &Chain<'a>) -> Option<usize> {
        let shift = chain.receiver_width(self.source);
        match self.max_shift {
            MaxShift::Cap(cap) => (shift <= cap.get()).then_some(shift),
            MaxShift::NoShift => None,
            MaxShift::Unlimited => Some(shift),
        }
    }

    /// True when `chain` carries more links than the cap allows or reads
    /// past `code_line_length` joined onto the row `start` lands on.
    fn trips(&self, chain: &Chain<'a>, start: TextSize) -> bool {
        let column = settled_column(self.reservations, start, || self.source.column_of(start));
        self.cap.is_some_and(|cap| chain.links.len() > cap)
            || column + chain.width(self.source) > self.code_line_length
    }

    /// Runs `walk` with `node` recorded as the enclosing parent.
    fn under(&mut self, node: impl Into<AnyNodeRef<'a>>, walk: impl FnOnce(&mut Self)) {
        let parent = std::mem::replace(&mut self.parent, node.into());
        walk(self);
        self.parent = parent;
    }

    /// Walks the receiver and each link's arguments, leaving the spine
    /// unvisited.
    fn walk_beside_spine(&mut self, chain: &Chain<'a>) {
        self.under(chain.receiver, |breaker| walk_expr(breaker, chain.receiver));
        for link in &chain.links {
            self.visit_arguments(&link.call.arguments);
        }
    }
}

impl<'a> AstVisitor<'a> for Breaker<'a> {
    fn visit_arguments(&mut self, arguments: &'a Arguments) {
        self.under(arguments, |breaker| walk_arguments(breaker, arguments));
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let Some(chain) = Chain::of(self.source, expr) else {
            self.under(expr, |breaker| walk_expr(breaker, expr));
            return;
        };
        match self.break_chain(expr, &chain) {
            Some(edit) => insert_edit(&mut self.edits, edit),
            None => self.walk_beside_spine(&chain),
        }
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.under(stmt, |breaker| walk_stmt(breaker, stmt));
    }
}
