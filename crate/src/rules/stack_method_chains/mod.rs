//! Breaks a fluent method chain across lines under two triggers, the
//! count trigger on a chain carrying more than `max_links` links and
//! the length trigger on one whose joined single-line form crosses
//! `code_line_length` from the column it lands at. The broken chain
//! sits inside a parenthesis pair, its head holding the receiver and
//! the first link and every later link hanging beneath the head's own
//! dot, a receiver wider than `max_shift` taking the full split. Both
//! measures read the settled form, so a hand-wrapped link counts at
//! the width `reflow_calls` closes it to, and a chain inside a broken
//! chain's receiver or argument breaks in the same text where it trips
//! from the column the break lands it at. Neither trigger reaches a
//! replacement field, a comment span, or a segment holding its break.
//! `spine` divides a chain and `render` builds the replacement.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_text_size::TextRange;

use crate::{
    config::{Config, MaxShift},
    primitives::{
        call_keywords::module_call_params,
        edit::{insert_edit, narrowed_replacement, singleton_groups},
        fracture,
        inline::{display_width, end_column},
        layout::item_indent,
        reserve,
        walk::{
            Descent, ParentedCollector, ParentedProbe, walk_parented_arguments, walk_parented_expr,
            walk_parented_exprs,
        },
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
    rejoin: fracture::Settings<'static>,
    reservations: reserve::Reservations,
}

impl StackMethodChains {
    pub(crate) const MESSAGE: &'static str = "break a long method chain to one link per line";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

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
        let targets = module_call_params(source);
        let reservations = source.columns(self.reservations);
        let mut breaker = Breaker {
            cap: self.max_links,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            max_shift: self.max_shift,
            rejoin: self.rejoin.against(&targets),
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
    rejoin: fracture::Settings<'a>,
    reservations: &'a reserve::Columns,
    source: &'a Source,
}

impl<'a> Breaker<'a> {
    /// The text breaking `chain` across lines from `column` on a row
    /// indented `indent`, `range` covering the grouping pair the source
    /// already carries, or `None` where neither trigger fires or a
    /// comment or a line-spanning segment holds the shape.
    fn broken(
        &self,
        expr: &'a Expr,
        chain: &Chain<'a>,
        range: TextRange,
        column: usize,
        indent: usize,
    ) -> Option<String> {
        if self.source.intersects_comment(range) {
            return None;
        }
        let joins = self.rejoin.joins(self.source, expr);
        if chain.spans_lines(self.source, &joins) || !self.trips(chain, column, &joins) {
            return None;
        }
        let text = render::broken(
            self.source,
            chain,
            indent,
            self.hang(chain),
            |segment, column, indent| self.segment(chain, segment, column, indent, &joins),
        );
        Some(text)
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

    /// The outermost chains inside `chain`'s receiver, or inside the
    /// argument list of its link at `segment` less one, each with the
    /// node enclosing it.
    fn nested(
        &self,
        chain: &Chain<'a>,
        segment: usize,
    ) -> Vec<(&'a Expr, AnyNodeRef<'a>, Chain<'a>)> {
        let source = self.source;
        let mut nested =
            ParentedCollector::new(Descent::Over, Descent::Over, |expr: &'a Expr, parent| {
                outermost_chain(source, expr, parent).map(|chain| (expr, parent, chain))
            });
        match segment.checked_sub(1) {
            None => walk_parented_expr(
                chain.receiver,
                chain.calls[0].func.as_ref().into(),
                &mut nested,
            ),
            Some(link) => walk_parented_arguments(chain.calls[link], &mut nested),
        }
        nested.found
    }

    /// `chain`'s segment at `segment` settled and written from `column`
    /// on a row indented `indent`, the receiver at index zero and each
    /// link past that, every chain inside it broken where it trips from
    /// the column it lands at. A segment whose settled row overflows
    /// the budget has its argument list exploded by `reflow_calls`, so
    /// a chain inside that list that fits one indent step past the row
    /// stays joined for that explode to seat, and one that trips even
    /// there breaks from the column the joined row reaches.
    fn segment(
        &self,
        chain: &Chain<'a>,
        segment: usize,
        column: usize,
        indent: usize,
        joins: &fracture::Joins,
    ) -> String {
        let range = segment
            .checked_sub(1)
            .map_or(chain.receiver_range, |link| chain.links[link]);
        let explodes = self.rejoin.closes()
            && column + display_width(&joins.settled(self.source, range)) > self.code_line_length;
        let mut out = String::new();
        let mut cursor = range.start();
        for (expr, parent, nested) in self.nested(chain, segment) {
            let nested_range = self.source.paren_aware_range(expr.into(), parent);
            out.push_str(&joins.settled(self.source, TextRange::new(cursor, nested_range.start())));
            let landing = end_column(&out, column);
            let seated = explodes && !self.trips(&nested, item_indent(indent), joins);
            match self
                .broken(expr, &nested, nested_range, landing, indent)
                .filter(|_| !seated)
            {
                Some(text) => out.push_str(&text),
                None => out.push_str(&joins.settled(self.source, nested_range)),
            }
            cursor = nested_range.end();
        }
        out.push_str(&joins.settled(self.source, TextRange::new(cursor, range.end())));
        out
    }

    /// True when `chain` carries more links than the cap allows or reads
    /// past `code_line_length` settled onto a row from `column`.
    fn trips(&self, chain: &Chain, column: usize, joins: &fracture::Joins) -> bool {
        self.cap.is_some_and(|cap| chain.links.len() > cap)
            || column + chain.width(self.source, joins) > self.code_line_length
    }
}

impl<'a> ParentedProbe<'a> for Breaker<'a> {
    const INTERPOLATIONS: Descent = Descent::Over;

    fn probe(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>, _: &[AnyNodeRef<'a>]) -> Descent {
        let Some(chain) = outermost_chain(self.source, expr, parent) else {
            return Descent::Into;
        };
        let range = self.source.paren_aware_range(expr.into(), parent);
        let column = self.reservations.column_in(self.source, range.start());
        let indent = self.source.line_indent_width(range.start());
        let Some(edit) = self
            .broken(expr, &chain, range, column, indent)
            .and_then(|text| narrowed_replacement(self.source, range, text))
        else {
            return Descent::Into;
        };
        insert_edit(&mut self.edits, edit);
        Descent::Over
    }
}

/// The chain `expr` opens, `None` where it opens none or where
/// `parent` already places it on the spine of a longer chain, an
/// attribute's value or a call's callee, so the outermost chain is the
/// one a break reshapes.
fn outermost_chain<'a>(source: &Source, expr: &'a Expr, parent: AnyNodeRef) -> Option<Chain<'a>> {
    if matches!(
        parent,
        AnyNodeRef::ExprAttribute(_) | AnyNodeRef::ExprCall(_)
    ) {
        return None;
    }
    Chain::of(source, expr)
}
