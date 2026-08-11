//! Explodes a call to one argument per line under three triggers. The
//! count trigger fires on a keyword-expressible call carrying more than
//! `max_args` arguments, rendering one keyword per line. The length
//! trigger fires on any call whose inline argument list crosses
//! `code_line_length` from the column that list lands at, exploding a
//! keyword-expressible call in keyword form and any other call
//! positionally. The span trigger fires on any call one of whose
//! arguments still spans rows once every closable fracture inside the
//! list shuts and hangs from its own row rather than from a column
//! inside it, whatever the argument count and whatever the joined width
//! would have been. The closing `)` drops to the indent of the row
//! carrying the argument list's `(`, a nested call in an argument value
//! explodes in the same pass, and a chained call settles its receiver
//! before the link that carries it, so every link measures the column it
//! lands at. No trigger reaches a call inside an f-string or t-string.
//! Order, `=` alignment, and trailing commas stay with `alphabetize`,
//! `align_equals`, and `strip_trailing_commas`.
//!
//! Where no trigger fires, an argument list the author fractured
//! rejoins onto one row, measured across the column its `(` lands at,
//! the joined arguments, and the text trailing the call to the end of
//! its logical line. A list carrying the flush column shape the
//! explode path emits holds its break instead, the same reading
//! `collection_layout` gives a literal.
//!
//! `measure` answers the column a construct reaches and the width it
//! reads, and `render` builds the text that replaces an argument list.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, InterpolatedStringElement, Parameters, Stmt,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};

use crate::{
    config::Config,
    primitives::{
        call_keywords::module_call_params,
        edit::{insert_edit, narrowed_replacement, singleton_groups},
        fracture, one_row, reserve,
        walk::walk_stmt,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod measure;
mod render;

pub(crate) struct CallLayout {
    code_line_length: usize,
    max_args: Option<usize>,
    one_row: one_row::Settings<'static>,
    rejoin: fracture::Settings<'static>,
    reservations: reserve::Reservations,
}

impl CallLayout {
    pub(crate) const MESSAGE: &'static str = "explode call arguments to one keyword per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            max_args: config.rules.call_layout.max_args.cap(),
            one_row: config.one_row_settings(),
            rejoin: config.fracture_settings(),
            reservations: config.equals_reservations(),
        }
    }
}

impl Rule for CallLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let reservations = self.reservations.columns(source);
        let mut exploder = Exploder {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            indent: None,
            line_shift: 0,
            max_args: self.max_args,
            one_row: self.one_row.against(&targets),
            origin: TextSize::new(0),
            origin_column: 0,
            rejoin: self.rejoin.against(&targets),
            reservations: &reservations,
            source,
            targets: &targets,
        };
        exploder.visit_body(&source.ast().body);
        singleton_groups(exploder.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Walks a module, or one relocated argument value, emitting the explode
/// edits its calls need. `origin` and `origin_column` place the walked
/// subtree's opening line, `line_shift` the columns every later line
/// moves by, and `indent` is the indent an exploded closing `)` drops to,
/// unset where each call answers to its own source line.
struct Exploder<'a> {
    code_line_length: usize,
    edits: Vec<Edit>,
    indent: Option<usize>,
    line_shift: isize,
    max_args: Option<usize>,
    one_row: one_row::Settings<'a>,
    origin: TextSize,
    origin_column: usize,
    rejoin: fracture::Settings<'a>,
    reservations: &'a reserve::Columns,
    source: &'a Source,
    targets: &'a HashMap<TextSize, &'a Parameters>,
}

impl<'a> AstVisitor<'a> for Exploder<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let Expr::Call(call) = expr else {
            walk_expr(self, expr);
            return;
        };
        // The callee settles first, so the argument list measures against
        // the row a reshaped receiver leaves it on.
        self.visit_expr(&call.func);
        let indent = self.indent_for(call);
        let column = self.open_paren_column(call, &self.callee_text(call));
        // The rendered list already carries every nested reshape, so a
        // walk into the arguments would decide the same text twice, the
        // second reading measuring against columns the first one set.
        if let Some(text) = self.explode_args(call, indent, column) {
            if let Some(edit) = narrowed_replacement(self.source, call.arguments.range(), text) {
                insert_edit(&mut self.edits, edit);
            }
            return;
        }
        self.visit_arguments(&call.arguments);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        walk_stmt(self, stmt);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, parse};

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
            CallLayout::from_config(&Config::default())
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
        let edits = CallLayout::from_config(&config)
            .apply(&source)
            .into_iter()
            .flatten()
            .collect();
        let text = applied_text(&source, edits);
        // The doubly-nested `wrap` answers the row it lands on, so it
        // explodes in this pass rather than the next one.
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
        let edits = CallLayout::from_config(&Config::default())
            .apply(&source)
            .into_iter()
            .flatten()
            .collect();
        let text = applied_text(&source, edits);
        // The call explodes, yet the string-bearing list stays at the floor,
        // its rows unshifted so the string interior keeps its column.
        assert!(
            text.contains("    note=[\n    \"x\","),
            "string-bearing value should not re-indent:\n{text}",
        );
    }
}
