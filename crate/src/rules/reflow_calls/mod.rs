//! Explodes a call to one argument per line under three triggers: the
//! count trigger on a keyword-expressible call past `max_args`, the
//! length trigger on a call whose inline argument list crosses
//! `code_line_length` from the column it lands at, and the span
//! trigger on a call an argument of which still spans rows once every
//! closable fracture inside the list shuts. The closing `)` drops to
//! the indent of the row carrying the `(`, a nested call explodes in
//! the same pass, and a chained call settles its receiver first. No
//! trigger reaches a call inside an f-string or t-string, or inside a
//! signature `reflow-signatures` lays out one parameter per line.
//! Where no trigger fires, a fractured list rejoins onto one row,
//! whereas the flush column shape holds its break. `measure` answers
//! the columns a decision reads beside the seat `stack_method_chains`
//! measures a relocated chain from, and `render` builds the replacement.

use std::cell::RefCell;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    primitives::{
        call_keywords::{CallTargets, module_call_params},
        edit::{apply_inline_edits, insert_edit, narrowed_replacement, singleton_groups},
        layout::is_layoutable,
        one_row, padding, reserve,
        travel::{Landing, block_shift, shifted_block, spans_a_string_part},
        walk::walk_stmt,
    },
    rules::{
        Rule, RuleId, alphabetize_siblings::Reorders, prefer_fstring::PreferFstring,
        reflow_signatures,
    },
    source::Source,
};

mod measure;
mod render;

#[derive(Debug)]
pub(crate) struct ReflowCalls {
    expands_literals: bool,
    fstrings: PreferFstring,
    one_row: one_row::Settings<'static>,
    reorders: Reorders,
    reservations: reserve::Reservations,
    signatures: reflow_signatures::Terms,
    stranding: padding::Stranding,
}

impl ReflowCalls {
    pub(crate) const MESSAGE: &'static str = "lay out a call's arguments against the line budget";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) const PRESERVES_TREE: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let collections = &config.rules.reflow_collections;
        Self {
            expands_literals: collections.enabled && collections.explode,
            fstrings: config.fstrings(),
            one_row: config.one_row_settings(),
            reorders: config.reorders(),
            reservations: config.equals_reservations(),
            signatures: reflow_signatures::Terms::from_config(config),
            stranding: config.stranded_padding(),
        }
    }

    /// Walks `source` and returns the edits that explode or rejoin its
    /// argument lists, recording each seat into `seats` when one is given.
    fn walk(
        &self,
        source: &Source,
        seats: Option<&RefCell<FxHashMap<TextRange, Seat>>>,
    ) -> Vec<Edit> {
        let targets = module_call_params(source);
        let reservations = source.columns(self.reservations);
        let rewrites = source.fstring_rewrites(self.fstrings);
        let stranded = source.stranded_padding(self.stranding);
        let padding = padding::beside(&stranded, &rewrites);
        let held = self
            .signatures
            .over(source, &targets, &padding, &rewrites)
            .exploding_parameters(&source.ast().body);
        let mut exploder = Exploder {
            edits: Vec::new(),
            expands_literals: self.expands_literals,
            held: &held,
            indent: None,
            line_shift: 0,
            one_row: self.one_row.against(&targets).forecasting(&rewrites),
            origin_column: 0,
            padding: &padding,
            region: source.module_range(),
            reorders: self.reorders,
            reservations: &reservations,
            seats,
            source,
            tail: 0,
            targets: &targets,
        };
        exploder.visit_body(&source.ast().body);
        exploder.edits
    }

    /// The seat of every call and attribute access inside an argument
    /// this rule's walk over `source` relocates, keyed by its range, less
    /// one inside an argument list a skip directive holds for this rule,
    /// which never moves, or inside a literal `reflow-collections` expands
    /// or a replacement field, where the walk does not reach.
    pub(crate) fn seats(&self, source: &Source) -> FxHashMap<TextRange, Seat> {
        let seats = RefCell::default();
        self.walk(source, Some(&seats));
        seats.into_inner()
    }
}

impl Rule for ReflowCalls {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(self.walk(source, None))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The terms one walk reshapes calls under, handed to a layout that
/// relocates an expression and reshapes the calls inside it.
#[derive(Clone, Copy)]
pub(crate) struct Reshaper<'a> {
    pub(crate) expands_literals: bool,
    pub(crate) one_row: one_row::Settings<'a>,
    pub(crate) padding: &'a [Edit],
    pub(crate) reorders: Reorders,
    pub(crate) reservations: &'a reserve::Columns,
    pub(crate) source: &'a Source,
    pub(crate) targets: &'a CallTargets<'a>,
}

impl<'a> Reshaper<'a> {
    /// `expr`'s text with every call inside it exploded once it lands
    /// per `landing`, its source `range` covering any grouping pair, an
    /// exploded closing `)` dropping to the landing indent and `tail`
    /// columns following the text on its last row. A block written
    /// across rows measures each call where its rows travel to and
    /// moves the rows with the result, one running through a
    /// row-spanning string part reshapes nothing, and `None` leaves the
    /// caller its own placement of the source slice.
    pub(crate) fn reshaped(
        self,
        expr: &'a Expr,
        range: TextRange,
        landing: Landing,
        tail: usize,
    ) -> Option<String> {
        let block = self.source.slice(range);
        let travel = if self.source.contains_line_break(range) {
            if spans_a_string_part(self.source, expr) {
                return None;
            }
            block_shift(self.source, block, &[], range.start(), landing)
        } else {
            None
        };
        // Rows render where the source wrote them, so a call on the
        // opening row renders one move short of the landing indent and
        // the shift below carries it there.
        let rows = travel.map_or(0, |travel| travel.rows);
        let mut exploder = Exploder {
            edits: Vec::new(),
            expands_literals: self.expands_literals,
            held: &[],
            indent: Some(landing.indent.saturating_add_signed(-rows)),
            line_shift: rows,
            one_row: self.one_row,
            origin_column: landing.column,
            padding: self.padding,
            region: range,
            reorders: self.reorders,
            reservations: self.reservations,
            seats: None,
            source: self.source,
            tail,
            targets: self.targets,
        };
        exploder.visit_expr(expr);
        if exploder.edits.is_empty() {
            return None;
        }
        let text = apply_inline_edits(self.source, range, &exploder.edits);
        Some(match travel {
            Some(travel) => shifted_block(&text, travel).into_owned(),
            None => text.into_owned(),
        })
    }
}

/// The position a call or attribute access takes once the walk
/// relocates the argument holding it. `column` is the column its start
/// reaches after every move, `indent` the indent its row is written at
/// before a later move carries that row `line_shift` columns, and
/// `tail` the columns trailing it on that row.
#[derive(Clone, Copy)]
pub(crate) struct Seat {
    pub(crate) column: usize,
    pub(crate) indent: usize,
    pub(crate) line_shift: isize,
    pub(crate) tail: usize,
}

/// Walks a module, or one relocated expression, emitting the explode
/// edits its calls need. `region` is the span the walk answers for and
/// `origin_column` the column its opening line lands at, `line_shift`
/// the columns every later line moves by, `tail` the columns the text
/// assembling the region writes after its last row, and `indent` is the
/// indent an exploded closing `)` drops to, unset where each call
/// answers to its own source line. `padding` is every edit
/// `strip-stranded-padding` emits over the source, `held` the start of
/// each parameter list `reflow-signatures` lays out one per line,
/// `expands_literals` whether `reflow-collections` expands an
/// overflowing literal, and `seats`, where set, collects the seat of
/// each call and attribute access inside a relocated region.
struct Exploder<'a> {
    edits: Vec<Edit>,
    expands_literals: bool,
    held: &'a [TextSize],
    indent: Option<usize>,
    line_shift: isize,
    one_row: one_row::Settings<'a>,
    origin_column: usize,
    padding: &'a [Edit],
    region: TextRange,
    reorders: Reorders,
    reservations: &'a reserve::Columns,
    seats: Option<&'a RefCell<FxHashMap<TextRange, Seat>>>,
    source: &'a Source,
    tail: usize,
    targets: &'a CallTargets<'a>,
}

impl<'a> AstVisitor<'a> for Exploder<'a> {
    /// Leaves a literal `reflow-collections` expands unwalked, the calls
    /// inside it reshaping where its entries land, and records the seat of
    /// each call and attribute access it reaches inside a relocated region
    /// into `seats`, where set.
    fn visit_expr(&mut self, expr: &'a Expr) {
        if is_layoutable(expr) && self.expands_later(expr) {
            return;
        }
        if let Some(seats) = self.seats
            && self.indent.is_some()
            && matches!(expr, Expr::Call(_) | Expr::Attribute(_))
        {
            seats
                .borrow_mut()
                .entry(expr.range())
                .or_insert_with(|| self.seat(expr));
        }
        let Expr::Call(call) = expr else {
            walk_expr(self, expr);
            return;
        };
        // The callee settles first, so the argument list measures against
        // the row a reshaped receiver leaves it on.
        self.visit_expr(&call.func);
        let column = self.open_paren_column(call);
        // The rendered list already carries every nested reshape, so the
        // arguments go unwalked.
        if let Some(text) = self.explode_args(call, column) {
            if let Some(edit) = narrowed_replacement(self.source, call.arguments.range(), text) {
                insert_edit(&mut self.edits, edit);
            }
            return;
        }
        self.visit_arguments(&call.arguments);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    /// Walks a `def` whose signature `reflow-signatures` lays out one
    /// parameter per line without its parameters or return annotation,
    /// the calls inside those reshaping where each parameter lands.
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt
            && self.held.binary_search(&fd.parameters.start()).is_ok()
        {
            for decorator in &fd.decorator_list {
                self.visit_decorator(decorator);
            }
            if let Some(type_params) = &fd.type_params {
                self.visit_type_params(type_params);
            }
            self.visit_body(&fd.body);
            return;
        }
        walk_stmt(self, stmt);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, at, parse};

    /// `source` with every edit the rule under `config` emits applied.
    fn applied(config: &Config, source: &Source) -> String {
        let edits = ReflowCalls::from_config(config)
            .apply(source)
            .into_iter()
            .flatten()
            .collect();
        applied_text(source, edits)
    }

    #[rstest]
    fn a_call_inside_a_replacement_field_emits_no_edit(#[values("f", "t")] prefix: &str) {
        // The narration runs long enough that the call inside it clears
        // the width trigger from its own column.
        let src = format!(
            "value = {prefix}\"a fairly long narration wrapping the gathered \
             values {{gather(alpha, beta, gamma)}}\"\n"
        );
        let source = parse(&src);
        assert!(
            ReflowCalls::from_config(&Config::default())
                .apply(&source)
                .is_empty(),
            "replacement field should emit no edit:\n{src}",
        );
    }

    #[test]
    fn call_two_levels_inside_a_collection_value_measures_where_it_lands() {
        let src =
            "emit(alpha=1, beta=[\n    helper(aaaa, wrap(bbbbbb, cccccc)),\n], gamma=3, delta=4)\n";
        let source = parse(src);
        let config = Config {
            code_line_length: NonZeroUsize::new(30),
            ..Config::default()
        };
        let text = applied(&config, &source);
        // The doubly-nested `wrap` answers the row it lands on, so it
        // explodes in this pass.
        assert!(
            text.contains("wrap(\n"),
            "nested call should explode in one pass:\n{text}",
        );
    }

    #[test]
    fn keyword_value_spanning_a_multiline_string_holds_the_floor() {
        let src =
            "emit(alpha=1, beta=2, gamma=3, note=[\n    \"x\",\n    \"\"\"multi\nline\"\"\",\n])\n";
        let source = parse(src);
        let text = applied(&Config::default(), &source);
        // The call explodes, yet the string-bearing list stays at the floor,
        // its rows unshifted so the string interior keeps its column.
        assert!(
            text.contains("    note=[\n    \"x\","),
            "string-bearing value should not re-indent:\n{text}",
        );
    }

    #[rstest]
    #[case::argument_on_the_exploded_row(
        "result = advise(alpha, beta, gamma.get(key).strip())\n",
        "gamma.get(key).strip()",
        Some((4, 4, 0, 0))
    )]
    #[case::argument_past_the_text_ahead_of_it(
        "result = outer(alpha_value, inner(beta_value, gamma.get(key)))\n",
        "gamma.get(key)",
        Some((22, 4, 0, 1))
    )]
    #[case::attribute_access_reads_its_own_tail(
        "result = advise(alpha, beta, gamma.get(key).value)\n",
        "gamma.get(key).value",
        Some((4, 4, 0, 0))
    )]
    #[case::call_measured_with_the_text_after_it(
        "result = advise(alpha_value, int(gamma.get(key)[0]), beta)\n",
        "gamma.get(key)",
        Some((8, 4, 0, 5))
    )]
    #[case::call_inside_a_wider_expression_reads_its_own_tail(
        "result = advise(alpha, m.group().split(\".\")[0].strip())\n",
        "m.group().split(\".\")",
        Some((4, 4, 0, 10))
    )]
    #[case::call_on_a_moved_row(
        "result = advise(alpha_value, inner(\n    delta.get(key),\n))\n",
        "delta.get(key)",
        Some((8, 4, 4, 1))
    )]
    #[case::call_opening_a_moved_argument(
        "result = advise(alpha_value, inner(\n    delta.get(key),\n))\n",
        "inner(\n    delta.get(key),\n)",
        Some((4, 0, 4, 0))
    )]
    #[case::call_behind_an_exploded_closer(
        "total = explode(alpha, beta, gamma) + delta.get(key)\n",
        "delta.get(key)",
        None
    )]
    #[case::call_inside_a_skipped_list(
        "result = advise(alpha, beta, gamma.get(key).strip())  # prose: skip[reflow-calls]\n",
        "gamma.get(key).strip()",
        None
    )]
    #[case::call_the_walk_leaves_in_place("x = f(a.b().c())\n", "a.b().c()", None)]
    #[case::expanding_literal_left_unwalked(
        "result = advise(alpha_value, [beta_value, gamma.get(key).strip(), delta_value])\n",
        "gamma.get(key).strip()",
        None
    )]
    fn seats_place_each_call_where_its_relocated_argument_lands(
        #[case] src: &str,
        #[case] expression: &str,
        #[case] expected: Option<(usize, usize, isize, usize)>,
    ) {
        let config = Config {
            code_line_length: NonZeroUsize::new(40),
            ..Config::default()
        };
        assert_eq!(
            ReflowCalls::from_config(&config)
                .seats(&parse(src))
                .get(&at(src, expression))
                .map(|seat| (seat.column, seat.indent, seat.line_shift, seat.tail)),
            expected,
        );
    }
}
