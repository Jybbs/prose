//! The shape decision one source's signatures resolve under, read from
//! configuration and answered per definition.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{Stmt, StmtFunctionDef};
use ruff_text_size::{Ranged, TextSize};

use super::render::rendered_parts;
use crate::{
    config::Config,
    primitives::walk::filter_map_over_stmts,
    primitives::{call_keywords::CallTargets, one_row},
    source::Source,
};

/// The terms this rule lays a signature out under, resolved from
/// configuration, so a rule measuring a call inside a parameter reads
/// the same decision about the signature around it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Terms {
    code_line_length: usize,
    expands_literals: bool,
    max_params: Option<usize>,
    one_row: one_row::Settings<'static>,
}

impl Terms {
    pub(crate) fn from_config(config: &Config) -> Self {
        let collections = &config.rules.reflow_collections;
        Self {
            code_line_length: config.code_width(),
            expands_literals: collections.enabled && collections.explode,
            max_params: config.rules.reflow_signatures.max_params.cap(),
            one_row: config.one_row_settings(),
        }
    }

    /// These terms over one source, `targets` the map
    /// [`module_call_params`] builds for it and `padding` the edits
    /// `strip-stranded-padding` emits over it.
    pub(crate) fn over<'a>(
        self,
        source: &'a Source,
        targets: &'a CallTargets<'a>,
        padding: &'a [Edit],
    ) -> Expansion<'a> {
        Expansion {
            code_line_length: self.code_line_length,
            expands_literals: self.expands_literals,
            max_params: self.max_params,
            one_row: self.one_row.against(targets),
            padding,
            source,
        }
    }
}

/// The shape decision over one source: whether a signature lays out one
/// parameter per line or on one row.
#[derive(Clone, Copy)]
pub(crate) struct Expansion<'a> {
    pub(super) code_line_length: usize,
    pub(super) expands_literals: bool,
    pub(super) max_params: Option<usize>,
    pub(super) one_row: one_row::Settings<'a>,
    pub(super) padding: &'a [Edit],
    pub(super) source: &'a Source,
}

/// The shape a signature takes.
pub(super) enum Shape {
    /// One parameter per line.
    Expanded,
    /// One row, carrying the canonical `(` through `:` text.
    Inline(String),
}

impl Expansion<'_> {
    /// The start of every parameter list in `body` this rule lays out
    /// one per line, ascending.
    pub(crate) fn exploding_parameters(&self, body: &[Stmt]) -> Vec<TextSize> {
        filter_map_over_stmts(body, |stmt| {
            stmt.as_function_def_stmt()
                .filter(|fd| matches!(self.shape(fd), Some(Shape::Expanded)))
                .map(|fd| fd.parameters.start())
        })
    }

    /// The shape `fd`'s signature takes, `None` where a comment inside
    /// `()` pins the shape it has. The one-row reading answers whether
    /// every parameter reaches a single row at all, so a signature
    /// holding one that cannot is laid out one per line whatever its
    /// width would have been.
    pub(super) fn shape(&self, fd: &StmtFunctionDef) -> Option<Shape> {
        let params = &fd.parameters;
        let one = TextSize::from(1u32);
        if self
            .source
            .intersects_comment(params.range().add_start(one).sub_end(one))
        {
            return None;
        }
        let count_trips = self.max_params.is_some_and(|cap| params.len() > cap);
        let inline = rendered_parts(params, |p| {
            self.one_row.parameter_form(self.source, p).map(Cow::Owned)
        })
        .map(|parts| self.build_inline(fd, &parts));
        Some(match inline {
            Some(text) if !count_trips && self.inline_fits(fd, &text) => Shape::Inline(text),
            _ => Shape::Expanded,
        })
    }
}
