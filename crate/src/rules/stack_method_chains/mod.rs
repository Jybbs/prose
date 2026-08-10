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
//! comment, or one whose segments still carry a line break once the
//! fractures inside them close up.
//!
//! Both measures and the rendered rows read the settled form rather
//! than the source, so a link the author hand-wrapped counts and
//! renders at the width `reflow_calls` closes it to.
//!
//! `spine` divides a chain into its receiver and links, and `render`
//! builds the text that replaces it.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_text_size::TextSize;

use crate::{
    config::{Config, MaxShift},
    primitives::{
        edit::{insert_edit, narrowed_replacement, singleton_groups},
        fracture, reserve,
        walk::{Descent, ParentedProbe, is_interpolated_string, walk_parented_exprs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod render;
mod spine;

use spine::Chain;

pub(crate) struct StackMethodChains {
    code_line_length: usize,
    max_links: Option<usize>,
    max_shift: MaxShift,
    rejoin: fracture::Settings,
    reservations: reserve::Reservations,
}

impl StackMethodChains {
    pub(crate) const MESSAGE: &'static str = "break a long method chain to one link per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.stack_method_chains;
        Self {
            code_line_length: config.code_width(),
            max_links: rules.max_links.cap(),
            max_shift: rules.max_shift,
            rejoin: config.fracture_settings(),
            reservations: config.equals_reservations(),
        }
    }
}

impl Rule for StackMethodChains {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let reservations = self.reservations.columns(source);
        let mut breaker = Breaker {
            cap: self.max_links,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_shift: self.max_shift,
            rejoin: self.rejoin,
            reservations: &reservations,
            source,
        };
        walk_parented_exprs(source.ast(), &mut breaker);
        singleton_groups(breaker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Emits the break edit each over-long or over-count chain needs as the
/// parent-tracking walk reaches it.
struct Breaker<'a> {
    cap: Option<usize>,
    code_line_length: usize,
    edits: Vec<Edit>,
    max_shift: MaxShift,
    rejoin: fracture::Settings,
    reservations: &'a reserve::Columns,
    source: &'a Source,
}

impl Breaker<'_> {
    /// The edit breaking `chain` across lines, or `None` where neither
    /// trigger fires, a comment or a line-spanning segment holds the
    /// shape, or the chain already reads as the break's own output.
    /// `parent` resolves the grouping pair the source already carries.
    fn break_chain(&self, expr: &Expr, chain: &Chain, parent: AnyNodeRef) -> Option<Edit> {
        let range = self.source.paren_aware_range(expr.into(), parent);
        if self.source.intersects_comment(range) {
            return None;
        }
        let joins = self.rejoin.joins(self.source, expr);
        if chain.spans_lines(self.source, &joins) || !self.trips(chain, range.start(), &joins) {
            return None;
        }
        let indent = self.source.line_indent_width(range.start());
        let text = render::broken(self.source, chain, indent, self.hang(chain), &joins);
        narrowed_replacement(self.source, range, text)
    }

    /// The columns each link's dot hangs past the head's indent, `None`
    /// where the receiver runs wider than `max_shift` allows and the
    /// chain takes the full split.
    fn hang(&self, chain: &Chain) -> Option<usize> {
        let shift = chain.receiver_width(self.source);
        match self.max_shift {
            MaxShift::Cap(cap) => (shift <= cap.get()).then_some(shift),
            MaxShift::NoShift => None,
            MaxShift::Unlimited => Some(shift),
        }
    }

    /// True when `chain` carries more links than the cap allows or reads
    /// past `code_line_length` settled onto the row `start` lands on.
    fn trips(&self, chain: &Chain, start: TextSize, joins: &fracture::Joins) -> bool {
        let column = self.reservations.column_in(self.source, start);
        self.cap.is_some_and(|cap| chain.links.len() > cap)
            || column + chain.width(self.source, joins) > self.code_line_length
    }
}

impl<'a> ParentedProbe<'a> for Breaker<'a> {
    fn probe(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>, _: &[AnyNodeRef<'a>]) -> Descent {
        if is_interpolated_string(expr) {
            return Descent::Over;
        }
        let Some(edit) = Chain::of(self.source, expr)
            .filter(|_| !inside_a_chain(parent))
            .and_then(|chain| self.break_chain(expr, &chain, parent))
        else {
            return Descent::Into;
        };
        insert_edit(&mut self.edits, edit);
        Descent::Over
    }
}

/// True where `parent` already places the expression under visit on the
/// spine of a longer chain, an attribute's value or a call's callee, so
/// the outermost chain is the one the break reshapes.
fn inside_a_chain(parent: AnyNodeRef) -> bool {
    matches!(
        parent,
        AnyNodeRef::ExprAttribute(_) | AnyNodeRef::ExprCall(_)
    )
}
