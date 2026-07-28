//! Explodes a call to one argument per line under two triggers. The
//! count trigger fires on a keyword-expressible call carrying more than
//! `max_args` arguments, rendering one keyword per line. The length
//! trigger fires on any call whose inline argument list crosses
//! `code_line_length` from the column that list lands at, exploding a
//! keyword-expressible call in keyword form and any other call
//! positionally. The closing `)` drops to the indent of the row carrying
//! the call, a nested call in an argument value explodes in the same
//! pass, and a chained call settles its receiver before the link that
//! carries it, so every link measures the column it lands at. Order,
//! `=` alignment, and trailing commas stay with `alphabetize`,
//! `align_equals`, and `strip_trailing_commas`.
//!
//! `measure` answers the column a construct reaches and the width it
//! reads, and `render` builds the text that replaces an argument list.

use std::collections::HashMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, Parameters,
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};

use crate::{
    config::Config,
    primitives::{
        aligner,
        call_keywords::module_call_params,
        edit::{insert_edit, narrowed_replacement, singleton_groups},
        reserve::reserved_columns,
    },
    rule::{Rule, RuleId},
    rules::align_equals::AlignEquals,
    source::Source,
};

mod measure;
mod render;

pub(crate) struct CallLayout {
    align_equals: Option<aligner::Settings>,
    code_line_length: usize,
    max_args: Option<usize>,
}

impl CallLayout {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            align_equals: AlignEquals::reserve_settings(config),
            code_line_length: config.code_width(),
            max_args: config.rules.call_layout.max_args.cap(),
        }
    }
}

impl Rule for CallLayout {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let reservations = reserved_columns(source, self.align_equals, AlignEquals::SLUG);
        let mut exploder = Exploder {
            cap: self.max_args,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            indent: None,
            line_shift: 0,
            origin: TextSize::new(0),
            origin_column: 0,
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
    cap: Option<usize>,
    code_line_length: usize,
    edits: Vec<Edit>,
    indent: Option<usize>,
    line_shift: isize,
    origin: TextSize,
    origin_column: usize,
    reservations: &'a HashMap<TextSize, usize>,
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
        let callee = self.callee_text(call);
        let indent = self.indent_for(call);
        let column = self.open_paren_column(call, &callee);
        if let Some(text) = self.explode_args(call, indent, column)
            && let Some(edit) = narrowed_replacement(self.source, call.arguments.range(), text)
        {
            insert_edit(&mut self.edits, edit);
            return;
        }
        self.visit_arguments(&call.arguments);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::testing::{applied_text, parse};

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
