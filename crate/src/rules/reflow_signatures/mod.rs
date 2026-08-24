//! Normalizes function signatures to a binary shape, one line or one
//! parameter per line, gated by `code_line_length`, `max_params`, and a
//! parameter whose annotation or default spans rows. Comments inside
//! `()` pin the existing shape. `terms` resolves the shape decision,
//! `measure` answers whether the one-line form fits, and `render`
//! builds the text each shape lands as.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Stmt, StmtFunctionDef,
    statement_visitor::{StatementVisitor, walk_stmt},
    token::TokenKind,
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    primitives::{
        call_keywords::module_call_params,
        edit::{narrowed_replacement, singleton_groups},
        layout::item_indent,
        padding, reserve,
        splice::splice_parses,
    },
    rule::{Rule, RuleId},
    rules::{alphabetize_siblings::Reorders, reflow_calls::Reshaper},
    source::Source,
};

mod measure;
mod render;
mod terms;

use render::{closing_parameter, rendered_parts};
use terms::Shape;
pub(crate) use terms::{Expansion, Terms};

pub(crate) struct ReflowSignatures {
    reorders: Reorders,
    reservations: reserve::Reservations,
    stranding: padding::Stranding,
    terms: Terms,
}

impl ReflowSignatures {
    pub(crate) const MESSAGE: &'static str =
        "normalize function signature to one-line or one-per-line shape";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            reorders: config.reorders(),
            reservations: config.equals_reservations(),
            stranding: config.stranded_padding(),
            terms: Terms::from_config(config),
        }
    }
}

impl Rule for ReflowSignatures {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let targets = module_call_params(source);
        let padding = source.stranded_padding(self.stranding);
        let reservations = source.columns(self.reservations);
        let expansion = self.terms.over(source, &targets, &padding);
        let mut visitor = Layout {
            edits: Vec::new(),
            expansion,
            newline: source.newline_str(),
            reshaper: Reshaper {
                expands_literals: expansion.expands_literals,
                one_row: expansion.one_row,
                padding: &padding,
                reorders: self.reorders,
                reservations: &reservations,
                source,
                targets: &targets,
            },
            source,
        };
        visitor.visit_body(&source.ast().body);
        singleton_groups(visitor.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Layout<'a> {
    edits: Vec<Edit>,
    expansion: Expansion<'a>,
    newline: &'static str,
    reshaper: Reshaper<'a>,
    source: &'a Source,
}

impl Layout<'_> {
    /// Emits one expand or collapse edit when `fd`'s signature
    /// diverges from the canonical inline-or-expanded form.
    fn process_def(&mut self, fd: &StmtFunctionDef) {
        let Some(shape) = self.expansion.shape(fd) else {
            return;
        };
        let params = &fd.parameters;
        let indent = self.source.line_indent_width(fd.start());
        let replacement_range = self.replacement_range(fd);
        let replacement = match shape {
            Shape::Expanded => {
                let item = item_indent(indent);
                let closing = closing_parameter(params);
                let parts = rendered_parts(params, |p| {
                    let tail = usize::from(closing != Some(p.range()));
                    Some(self.place(p, item, tail))
                })
                .expect("placing a parameter always renders");
                self.build_expanded(fd, &parts, indent)
            }
            Shape::Inline(text) if self.source.contains_line_break(replacement_range) => text,
            Shape::Inline(_) => return,
        };
        // Emit the reshape only when the spliced signature re-parses, the
        // safety net for return types the rewrite cannot reassemble.
        if splice_parses(
            self.source,
            fd.range(),
            replacement_range,
            &replacement,
            parse_module,
        ) {
            self.edits.extend(narrowed_replacement(
                self.source,
                replacement_range,
                replacement,
            ));
        }
    }

    /// Returns the range covering the signature's `(` through `:`,
    /// the surface this rule rewrites.
    ///
    /// # Panics
    ///
    /// Panics if `fd.body` is empty or the `:` token cannot be located
    /// between `)` and the body.
    fn replacement_range(&self, fd: &StmtFunctionDef) -> TextRange {
        let body_start = fd
            .body
            .first()
            .expect("function def has a non-empty body")
            .start();
        let colon = self
            .source
            .first_token_offset_in_range(
                TextRange::new(fd.parameters.range().end(), body_start),
                |t| t.kind() == TokenKind::Colon,
            )
            .expect("function def carries a `:` between `)` and the body");
        TextRange::new(fd.parameters.range().start(), colon + TextSize::from(1u32))
    }
}

impl<'a> StatementVisitor<'a> for Layout<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_def(fd);
        }
        walk_stmt(self, stmt);
    }
}
