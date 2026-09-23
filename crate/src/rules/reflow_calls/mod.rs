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
//! whereas the flush column shape holds its break. Within an
//! expression that `reflow-collections` moves, each collection literal,
//! subscript, or comprehension takes that rule's layout where it lands.
//! `measure` holds the column arithmetic behind each decision, and
//! `render` builds the replacement.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, InterpolatedStringElement, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        call_keywords::{CallTargets, module_call_params},
        edit::{apply_inline_edits, insert_edit, narrowed_replacement, singleton_groups},
        layout::is_collapsible,
        one_row, padding, reserve,
        travel::{Landing, block_shift, shifted_block, spans_a_string_part},
        walk::walk_stmt,
    },
    rules::{Rule, RuleId, alphabetize_siblings::Reorders, reflow_signatures},
    source::Source,
};

mod measure;
mod render;

/// The layout `reflow-collections` gives a collapsible construct, read
/// by a walk that relocates the expression holding it.
pub(crate) trait CollectionLayout {
    /// `expr`'s replacement once it lands at `column`, its closing
    /// bracket dropping to `indent` and `tail` columns following it, or
    /// `None` where it stays as written.
    fn laid_out(&self, expr: &Expr, column: usize, indent: usize, tail: usize) -> Option<String>;
}

#[derive(Debug)]
pub(crate) struct ReflowCalls {
    one_row: one_row::Settings<'static>,
    reorders: Reorders,
    reservations: reserve::Reservations,
    signatures: reflow_signatures::Terms,
    stranding: padding::Stranding,
}

impl ReflowCalls {
    pub(crate) const MESSAGE: &'static str = "lay out a call's arguments against the line budget";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            one_row: config.one_row_settings(),
            reorders: config.reorders(),
            reservations: config.equals_reservations(),
            signatures: reflow_signatures::Terms::from_config(config),
            stranding: config.stranded_padding(),
        }
    }
}

impl Rule for ReflowCalls {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let reservations = source.columns(self.reservations);
        let padding = source.stranded_padding(self.stranding);
        let held = self
            .signatures
            .over(source, &targets, &padding)
            .exploding_parameters(&source.ast().body);
        let mut exploder = Exploder {
            edits: Vec::new(),
            held: &held,
            indent: None,
            layout: None,
            line_shift: 0,
            one_row: self.one_row.against(&targets),
            origin_column: 0,
            padding: &padding,
            region: source.module_range(),
            reorders: self.reorders,
            reservations: &reservations,
            source,
            tail: 0,
            targets: &targets,
        };
        exploder.visit_body(&source.ast().body);
        singleton_groups(exploder.edits)
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

/// Walks a module, or one relocated expression, emitting the explode
/// edits its calls need. `region` is the span the walk answers for and
/// `origin_column` the column its opening line lands at, `line_shift`
/// the columns every later line moves by, `tail` the columns the text
/// assembling the region writes after its last row, and `indent` is the
/// indent an exploded closing bracket drops to, unset where each call
/// answers to its own source line. `padding` is every edit
/// `strip-stranded-padding` emits over the source, `held` the start of
/// each parameter list `reflow-signatures` lays out one per line, and
/// `layout` the layout a collapsible construct takes where the walk
/// reaches it, unset where `reflow-collections` walks the text later in
/// the fold.
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
    source: &'a Source,
    tail: usize,
    targets: &'a CallTargets<'a>,
}

impl<'a> AstVisitor<'a> for Exploder<'a> {
    /// Lays out each collapsible construct where it lands when `layout`
    /// is set, and otherwise leaves unwalked a literal
    /// `reflow-collections` expands later. The calls inside either one
    /// reshape where its entries land.
    fn visit_expr(&mut self, expr: &'a Expr) {
        match self.layout {
            Some(layout) if is_collapsible(expr) => {
                if let Some(text) = self.laid_out(layout, expr) {
                    if let Some(edit) = narrowed_replacement(self.source, expr.range(), text) {
                        insert_edit(&mut self.edits, edit);
                    }
                    return;
                }
            }
            None if self.expands_later(expr) => return,
            _ => {}
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
    use crate::testing::{applied_text, parse};

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
}
