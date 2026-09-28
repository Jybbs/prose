//! Explodes a call to one argument per line under three triggers: the
//! count trigger on a keyword-expressible call past `max_args`, the
//! length trigger on a call whose inline argument list crosses
//! `code_line_length` from the column it lands at, and the span
//! trigger on a call an argument of which still spans rows once every
//! closable fracture inside the list shuts. The closing `)` drops to
//! the indent of the row carrying the `(`, a nested call explodes in
//! the same pass, and a chained call settles its receiver first. No
//! trigger reaches a call inside an f-string or t-string, inside a `%`
//! or `str.format()` interpolation `prefer-fstring` converts on the row
//! it lands on, or inside a signature `reflow-signatures` lays out one
//! parameter per line.
//! Where no trigger fires, a fractured list rejoins onto one row,
//! whereas the flush column shape holds its break. Within an
//! expression that `reflow-collections` moves, each collection literal,
//! subscript, or comprehension takes that rule's layout where it lands.
//! `measure` answers the columns a decision reads beside the seat
//! `stack_method_chains` measures a relocated chain from, and `render`
//! builds the replacement.

use std::cell::RefCell;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, ExprCall, InterpolatedStringElement, Stmt,
    visitor::source_order::{self, SourceOrderVisitor},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    primitives::{
        call_keywords::{CallTargets, module_call_params},
        edit::{apply_inline_edits, singleton_groups},
        layout::is_collapsible,
        one_row, padding, reserve,
        slots::item_covering,
        travel::{Landing, block_shift, shifted_block, spans_a_string_part},
    },
    rules::{
        Rule, RuleId, alphabetize_siblings::Reorders, prefer_fstring::PreferFstring,
        reflow_signatures,
    },
    source::Source,
};

mod measure;
mod render;

/// The layout `reflow-collections` gives a collapsible construct, read
/// by a walk that relocates the expression holding it.
pub(crate) trait CollectionLayout {
    /// Returns `expr`'s replacement at `column`, or `None` where it stays
    /// as written. Its closing bracket drops to `indent`, and `tail`
    /// columns follow its last row.
    fn laid_out(&self, expr: &Expr, column: usize, indent: usize, tail: usize) -> Option<String>;
}

/// What one walk spans: the whole module, or the text over `region`
/// once it lands at `seat`, the walk visiting the argument list of
/// `call`, whose callee sits outside `region`, or the expression `expr`.
#[derive(Clone, Copy)]
pub(crate) enum Reach<'a> {
    Module,
    Arguments {
        call: &'a ExprCall,
        region: TextRange,
        seat: Seat,
    },
    Expr {
        expr: &'a Expr,
        region: TextRange,
        seat: Seat,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReflowCalls {
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
        Self {
            fstrings: config.fstrings(),
            one_row: config.one_row_settings(),
            reorders: config.reorders(),
            reservations: config.equals_reservations(),
            signatures: reflow_signatures::Terms::from_config(config),
            stranding: config.stranded_padding(),
        }
    }

    /// Walks what `reach` spans in `source` and returns the edits that
    /// explode or rejoin its argument lists, recording what it reaches
    /// into `seating` when one is given.
    fn walk(&self, source: &Source, reach: Reach, seating: Option<&RefCell<Seating>>) -> Vec<Edit> {
        let targets = module_call_params(source);
        let reservations = source.columns(&self.reservations);
        let rewrites = source.fstring_rewrites(self.fstrings);
        let stranded = source.stranded_padding(self.stranding);
        let padding = padding::beside(&stranded, &rewrites);
        let held = self
            .signatures
            .over(source, &targets, &padding, &rewrites)
            .exploding_parameters(&source.ast().body);
        let mut exploder = Exploder {
            edits: Vec::new(),
            held: &held,
            indent: None,
            layout: None,
            line_shift: 0,
            one_row: self.one_row.against(&targets).forecasting(&rewrites),
            origin_column: 0,
            padding: &padding,
            region: source.module_range(),
            reorders: self.reorders,
            reservations: &reservations,
            seating,
            seats_elements: false,
            source,
            tail: 0,
            targets: &targets,
        };
        match reach {
            Reach::Module => {
                exploder.visit_body(&source.ast().body);
                exploder.edits
            }
            Reach::Arguments { call, region, seat } => {
                let mut landed = exploder.landed(region, seat);
                landed.lay_out_arguments(call);
                landed.edits
            }
            Reach::Expr { expr, region, seat } => {
                let mut landed = exploder.landed(region, seat);
                landed.visit_expr(expr);
                landed.edits
            }
        }
    }

    /// True where `range` sits inside an interpolation this rule's walk
    /// over `source` leaves for `prefer-fstring` to convert, reading the
    /// source's walk only where a forecast rewrite covers `range`.
    pub(crate) fn converts(&self, source: &Source, range: TextRange) -> bool {
        self.forecasts(source, range) && source.call_seating(self).converts(range)
    }

    /// True where a rewrite `prefer-fstring` forecasts over `source`
    /// covers `range`, whether or not its f-string fits where it lands.
    pub(crate) fn forecasts(&self, source: &Source, range: TextRange) -> bool {
        item_covering(&source.fstring_rewrites(self.fstrings), range).is_some()
    }

    /// The [`Seating`] this rule's walk over what `reach` spans in
    /// `source` records, walked on each read.
    pub(crate) fn recorded(&self, source: &Source, reach: Reach) -> Seating {
        let seating = RefCell::default();
        self.walk(source, reach, Some(&seating));
        seating.into_inner()
    }

    /// The seat this rule's walk over `source` records for the call or
    /// attribute access spanning `range`, `None` where it records none.
    pub(crate) fn seat(&self, source: &Source, range: TextRange) -> Option<Seat> {
        source.call_seating(self).seat(range)
    }

    /// The [`Seating`] this rule's walk over `source` records, with no
    /// seat for a call inside an argument list a skip directive holds for
    /// this rule, which never moves, or inside a literal
    /// `reflow-collections` expands, an interpolation `prefer-fstring`
    /// converts, or a replacement field, where the walk does not reach.
    pub(crate) fn seating(&self, source: &Source) -> Seating {
        self.recorded(source, Reach::Module)
    }
}

impl Rule for ReflowCalls {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        singleton_groups(self.walk(source, Reach::Module, None))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The terms one walk reshapes calls under, handed to a layout that
/// relocates an expression and reshapes the calls inside it. `layout`
/// lays out each collapsible construct the walk reaches, and where it
/// is unset a literal `reflow-collections` expands is left to that
/// rule's own pass.
#[derive(Clone, Copy)]
pub(crate) struct Reshaper<'a> {
    pub(crate) layout: Option<&'a dyn CollectionLayout>,
    pub(crate) one_row: one_row::Settings<'a>,
    pub(crate) padding: &'a [Edit],
    pub(crate) reorders: Reorders,
    pub(crate) reservations: &'a reserve::Columns,
    pub(crate) source: &'a Source,
    pub(crate) targets: &'a CallTargets<'a>,
}

impl<'a> Reshaper<'a> {
    /// `expr`'s text once it lands per `landing`, with every call inside it
    /// exploded and, where `layout` is set, every collapsible construct laid
    /// out. `range` covers any grouping pair, an exploded closing bracket
    /// drops to the landing indent, and `tail` columns follow the last row.
    /// A block written across rows measures each call where its rows travel
    /// to and moves the rows with the result, one running through a
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
            held: &[],
            indent: Some(landing.indent.saturating_add_signed(-rows)),
            layout: self.layout,
            line_shift: rows,
            one_row: self.one_row,
            origin_column: landing.column,
            padding: self.padding,
            region: range,
            reorders: self.reorders,
            reservations: self.reservations,
            seating: None,
            seats_elements: false,
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
#[derive(Clone, Copy, Debug)]
pub(crate) struct Seat {
    pub(crate) column: usize,
    pub(crate) indent: usize,
    pub(crate) line_shift: isize,
    pub(crate) tail: usize,
}

/// What a walk over a source records: the seat of each call and
/// attribute access inside an argument it relocates, keyed by its
/// range, and the range of each expression it leaves unwalked inside an
/// interpolation `prefer-fstring` converts where it lands.
#[derive(Clone, Debug, Default)]
pub(crate) struct Seating {
    converted: Vec<TextRange>,
    seats: FxHashMap<TextRange, Seat>,
}

impl Seating {
    /// True where `range` sits inside an interpolation the walk leaves
    /// for `prefer-fstring` to convert.
    pub(crate) fn converts(&self, range: TextRange) -> bool {
        self.converted
            .iter()
            .any(|converted| converted.contains_range(range))
    }

    /// The seat of the call or attribute access spanning `range`, `None`
    /// where the walk records none.
    pub(crate) fn seat(&self, range: TextRange) -> Option<Seat> {
        self.seats.get(&range).copied()
    }
}

/// Walks a module, or one relocated expression, emitting the explode
/// edits its calls need. `region` is the span the walk answers for and
/// `origin_column` the column its opening line lands at, `line_shift`
/// the columns every later line moves by, `tail` the columns the text
/// assembling the region writes after its last row, and `indent` is the
/// indent an exploded closing bracket drops to, unset where each call
/// answers to its own source line. `padding` is every edit
/// `strip-stranded-padding` emits over the source merged with the
/// forecast `prefer-fstring` rewrites, `held` the start of
/// each parameter list `reflow-signatures` lays out one per line,
/// `layout` the layout a collapsible construct takes where the walk
/// reaches it, unset where `reflow-collections` walks the text later in
/// the fold, and `seating`, where set, collects what the walk records.
/// `seats_elements` is true for a walk landed at the seat a stacked
/// chain's segment takes, which seats the elements of each literal
/// `reflow-collections` expands later.
struct Exploder<'a> {
    edits: Vec<Edit>,
    held: &'a [TextSize],
    indent: Option<usize>,
    layout: Option<&'a dyn CollectionLayout>,
    line_shift: isize,
    one_row: one_row::Settings<'a>,
    origin_column: usize,
    padding: &'a [Edit],
    region: TextRange,
    reorders: Reorders,
    reservations: &'a reserve::Columns,
    seating: Option<&'a RefCell<Seating>>,
    seats_elements: bool,
    source: &'a Source,
    tail: usize,
    targets: &'a CallTargets<'a>,
}

impl<'a> SourceOrderVisitor<'a> for Exploder<'a> {
    /// Leaves unwalked an expression inside an interpolation
    /// `prefer-fstring` converts, whose calls land in replacement fields,
    /// and otherwise lays out each collapsible construct where it lands
    /// when `layout` is set, leaving unwalked one it leaves as written
    /// that [`Settings::holds_its_row`](one_row::Settings::holds_its_row)
    /// holds, and otherwise leaves a literal `reflow-collections` expands
    /// later to that rule, seating its elements alone. Records into
    /// `seating`, where set, each expression it leaves for
    /// `prefer-fstring` and the seat of each call and attribute access it
    /// reaches inside a relocated region.
    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.converts(expr) {
            if let Some(seating) = self.seating {
                seating.borrow_mut().converted.push(expr.range());
            }
            return;
        }
        match self.layout {
            Some(layout) if is_collapsible(expr) => {
                if let Some(text) = self.laid_out(layout, expr) {
                    self.replace(expr.range(), text);
                    return;
                }
                if self.one_row.holds_its_row(self.source, expr) {
                    return;
                }
            }
            None if self.expands_later(expr) => {
                self.seat_elements(expr);
                return;
            }
            _ => {}
        }
        if let Some(seating) = self.seating
            && self.indent.is_some()
            && matches!(expr, Expr::Call(_) | Expr::Attribute(_))
        {
            seating
                .borrow_mut()
                .seats
                .entry(expr.range())
                .or_insert_with(|| self.seat(expr));
        }
        let Expr::Call(call) = expr else {
            source_order::walk_expr(self, expr);
            return;
        };
        // The callee settles first, so the argument list measures against
        // the row a reshaped receiver leaves it on.
        self.visit_expr(&call.func);
        self.lay_out_arguments(call);
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
        source_order::walk_stmt(self, stmt);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;
    use ruff_python_ast::PythonVersion;

    use super::*;
    use crate::testing::{applied_text, at, first_value, parse};

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

    #[rstest]
    #[case::one_column_past_the_budget(73, true)]
    #[case::at_the_budget(74, false)]
    fn a_call_inside_a_template_explodes_only_where_its_fstring_overflows(
        #[case] width: usize,
        #[case] explodes: bool,
    ) {
        // The f-string `prefer-fstring` forecasts for the template runs
        // its row to 74 columns, so `describe(` explodes where it is
        // written only under a narrower budget.
        let source = parse(
            "message = \"%s: %s\" % (describe(first_argument, second_argument), trailing_value)\n",
        );
        let mut config = Config {
            code_line_length: NonZeroUsize::new(width),
            target_version: Some(PythonVersion::PY314),
            ..Config::default()
        };
        config.rules.reflow_collections.enabled = false;
        let text = applied(&config, &source);
        assert_eq!(text.contains("describe(\n"), explodes, "{text}");
    }

    #[rstest]
    #[case::one_column_past_the_budget(65, true)]
    #[case::at_the_budget(66, false)]
    fn a_template_measures_its_fstring_with_the_text_trailing_its_row(
        #[case] width: usize,
        #[case] explodes: bool,
    ) {
        // The f-string `prefer-fstring` forecasts for the `str.format()`
        // call runs its row to 51 columns alone and to 66 with
        // ` + suffix_value` trailing it.
        let source =
            parse("x = \"{}!\".format(describe(first_argument, second_argument)) + suffix_value\n");
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            target_version: Some(PythonVersion::PY314),
            ..Config::default()
        };
        let text = applied(&config, &source);
        assert_eq!(text.contains(".format(\n"), explodes, "{text}");
    }

    #[rstest]
    #[case::reflow_collections_expands_the_literal(
        "x = [helper(a=1, b=2, c=3, d=4), b]\n",
        true,
        false
    )]
    #[case::reflow_collections_off("x = [helper(a=1, b=2, c=3, d=4), b]\n", false, true)]
    #[case::reflow_collections_held_by_a_skip(
        "x = [helper(a=1, b=2, c=3, d=4), b]  # prose: skip[reflow-collections]\n",
        true,
        true
    )]
    fn a_literal_holding_a_count_exploded_call_is_left_to_reflow_collections(
        #[case] src: &str,
        #[case] collections: bool,
        #[case] edits: bool,
    ) {
        let source = parse(src);
        let mut config = Config::default();
        config.rules.reflow_collections.enabled = collections;
        assert_eq!(
            !ReflowCalls::from_config(&config).apply(&source).is_empty(),
            edits
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
    #[case::fstring_fits_its_landing_row(39, true)]
    #[case::fstring_overflows_its_landing_row(38, false)]
    fn seating_marks_a_chain_inside_a_template_left_for_prefer_fstring(
        #[case] width: usize,
        #[case] converts: bool,
    ) {
        // `advise(` explodes at either width, and the f-string lands on
        // its own row at 39 columns.
        let src = "result = advise(alpha_value, beta_value, \"%s:%s\" % (gamma.get(key).strip(), delta))\n";
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            target_version: Some(PythonVersion::PY314),
            ..Config::default()
        };
        assert_eq!(
            ReflowCalls::from_config(&config)
                .seating(&parse(src))
                .converts(at(src, "gamma.get(key).strip()")),
            converts,
        );
    }

    #[test]
    fn landed_walk_seats_the_elements_of_a_literal_expanded_later() {
        let src =
            "result = advise(alpha_value, [beta_value, gamma.get(key).strip(), delta_value])\n";
        let config = Config {
            code_line_length: NonZeroUsize::new(40),
            ..Config::default()
        };
        let source = parse(src);
        let reflow_calls = ReflowCalls::from_config(&config);
        let call = first_value(&source)
            .as_call_expr()
            .expect("the value is a call");
        let seat = Seat {
            column: 9,
            indent: 0,
            line_shift: 0,
            tail: 0,
        };
        let landed = reflow_calls.recorded(
            &source,
            Reach::Arguments {
                call,
                region: call.range(),
                seat,
            },
        );
        assert_eq!(
            landed.seat(at(src, "gamma.get(key).strip()")).map(|seat| (
                seat.column,
                seat.indent,
                seat.line_shift,
                seat.tail
            )),
            Some((8, 8, 0, 0)),
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
        Some((4, 4, 0, 11))
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
                .seating(&parse(src))
                .seat(at(src, expression))
                .map(|seat| (seat.column, seat.indent, seat.line_shift, seat.tail)),
            expected,
        );
    }
}
