//! Normalizes function signatures to a binary shape, one line or one
//! parameter per line, gated by `code_line_length`, `max_params`, and a
//! parameter whose annotation or default spans rows while hanging from
//! the parameter's own row. Comments inside `()` pin the existing shape.
//! The one-line form measures at the width the later rules settle it
//! to, the padding `strip-stranded-padding` drops inside a parameter
//! coming off, and a literal inside the return annotation that
//! `reflow-collections` expands ending the opening row at its bracket.
//! A parameter laid out on its own row carries every call inside its
//! annotation and default reshaped where that row lands, the reading
//! `reflow-calls` leaves to this rule for a signature it expands.

use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, Expr, Parameters, Stmt, StmtFunctionDef,
    statement_visitor::{StatementVisitor, walk_stmt},
    token::TokenKind,
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        call_keywords::{CallTargets, module_call_params},
        edit::{narrowed_replacement, singleton_groups},
        inline::{end_column, opening_width},
        layout::{Separator, explode_parens, item_indent},
        one_row, padding,
        range::return_annotation_range,
        reserve,
        splice::splice_parses,
        travel::{Landing, placed_block},
        walk::{Descent, ParentedProbe, filter_map_over_stmts, walk_parented_expr},
    },
    rule::{Rule, RuleId},
    rules::{alphabetize_siblings::Reorders, reflow_calls::Reshaper},
    source::Source,
};

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

/// The terms this rule lays a signature out under, resolved from
/// configuration, so a rule measuring a call inside a parameter reads
/// the same decision about the signature around it.
#[derive(Clone, Copy)]
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
    code_line_length: usize,
    expands_literals: bool,
    max_params: Option<usize>,
    one_row: one_row::Settings<'a>,
    padding: &'a [Edit],
    source: &'a Source,
}

/// The shape a signature takes.
enum Shape {
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
    fn shape(&self, fd: &StmtFunctionDef) -> Option<Shape> {
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

    /// Builds the canonical inline text spanning `(` through `:` from
    /// `parts`.
    fn build_inline(&self, fd: &StmtFunctionDef, parts: &[Cow<str>]) -> String {
        let mut out = format!("({})", parts.join(", "));
        self.push_return_and_colon(&mut out, fd);
        out
    }

    /// True when the inline signature `text` sits inside the budget at
    /// the width the later rules settle it to: the padding
    /// `strip-stranded-padding` drops inside each parameter comes off,
    /// and where the row still overflows, the first literal along the
    /// row whose one-row form overflows from its column is the one
    /// `reflow-collections` expands. One inside a parameter leaves that
    /// parameter spanning rows, so the one-line form is out of reach,
    /// whereas one inside the return annotation ends the opening row
    /// at its bracket.
    fn inline_fits(&self, fd: &StmtFunctionDef, text: &str) -> bool {
        let start = fd.parameters.range().start();
        let slack_before = |offset: TextSize| -> isize {
            fd.parameters
                .iter()
                .filter(|param| param.end() <= offset)
                .map(|param| padding::slack(self.source, self.padding, param.range()))
                .sum()
        };
        let width = opening_width(text).saturating_add_signed(-slack_before(fd.end()));
        if !self
            .source
            .column_overflows(start, width, self.code_line_length)
        {
            return true;
        }
        if !self.expands_literals {
            return false;
        }
        let returns = fd.returns.as_deref();
        for (literal, parent, head) in self.inline_literals(fd, text) {
            let column = self.source.column_of(start)
                + text[..head]
                    .width()
                    .saturating_add_signed(-slack_before(literal.start()));
            let tail = text[head + self.source.slice(literal).len()..].width();
            if self
                .one_row
                .fitted(self.source, literal, parent, column, tail)
                .is_some()
            {
                continue;
            }
            let in_returns = returns.is_some_and(|ret| ret.range().contains_range(literal.range()));
            return in_returns && column < self.code_line_length;
        }
        false
    }

    /// Every literal inside the inline signature `text` in source
    /// order, each with the node enclosing it and the offset its source
    /// text opens at inside `text`. A parameter whose rendered form
    /// departs from its source slice contributes none, its literals
    /// sitting at no offset the slice locates.
    fn inline_literals<'f>(
        &self,
        fd: &'f StmtFunctionDef,
        text: &str,
    ) -> Vec<(&'f Expr, AnyNodeRef<'f>, usize)> {
        let mut literals = Vec::new();
        let mut cursor = 0;
        for param in fd.parameters.iter() {
            let slice = self.source.slice(param.range());
            let Some(found) = text[cursor..].find(slice) else {
                continue;
            };
            let base = cursor + found;
            cursor = base + slice.len();
            let inner = param.as_parameter();
            let mut sites: Vec<(&Expr, AnyNodeRef)> = Vec::new();
            if let Some(annotation) = inner.annotation.as_deref() {
                sites.push((annotation, inner.into()));
            }
            if let AnyParameterRef::NonVariadic(slot) = param
                && let Some(default) = slot.default.as_deref()
            {
                sites.push((default, slot.into()));
            }
            for (expr, parent) in sites {
                for (literal, enclosing) in literals_beneath(expr, parent) {
                    let offset = (literal.start() - param.start()).to_usize();
                    literals.push((literal, enclosing, base + offset));
                }
            }
        }
        if let Some(returns) = fd.returns.as_deref() {
            let annotation = return_annotation_range(returns, fd, self.source.tokens());
            // The text closes with the annotation's slice and `:`.
            let base = text.len() - 1 - annotation.len().to_usize();
            for (literal, enclosing) in literals_beneath(returns, fd.into()) {
                let offset = (literal.start() - annotation.start()).to_usize();
                literals.push((literal, enclosing, base + offset));
            }
        }
        literals
    }

    fn push_return_and_colon(&self, out: &mut String, fd: &StmtFunctionDef) {
        if let Some(ret) = fd.returns.as_deref() {
            out.push_str(" -> ");
            let range = return_annotation_range(ret, fd, self.source.tokens());
            out.push_str(self.source.slice(range));
        }
        out.push(':');
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
    /// Builds the canonical expanded text spanning `(` through `:` from
    /// `parts`, one parameter per line.
    fn build_expanded(&self, fd: &StmtFunctionDef, parts: &[Cow<str>], indent: usize) -> String {
        let mut out = explode_parens(
            self.newline,
            indent,
            parts.len(),
            |out, i| out.push_str(&parts[i]),
            Separator::Comma,
        );
        self.expansion.push_return_and_colon(&mut out, fd);
        out
    }

    /// `param`'s text placed at `indent` with `tail` columns closing its
    /// last row, every call inside its annotation and default reshaped
    /// where it lands, or the source text moved whole where none
    /// reshapes. A variadic parameter carries its `*` or `**` prefix and
    /// holds no default.
    fn place<'p>(&'p self, param: AnyParameterRef, indent: usize, tail: usize) -> Cow<'p, str> {
        self.reshaped(param, indent, tail).map_or_else(
            || {
                placed_block(
                    self.source,
                    param.range(),
                    Landing::own_row(param.start(), indent),
                )
            },
            Cow::Owned,
        )
    }

    /// `param`'s text at `indent` with the calls inside its annotation
    /// and its default reshaped where each lands, `tail` the columns
    /// closing the last row. Each site measures from the column the text
    /// ahead of it ends at and across the opening row of the text after
    /// it, and one no call inside reshapes moves whole. `None` where no
    /// site reshapes, or where text between two sites spans rows.
    fn reshaped(&self, param: AnyParameterRef, indent: usize, tail: usize) -> Option<String> {
        let inner = param.as_parameter();
        let mut sites: Vec<(&Expr, AnyNodeRef)> = Vec::new();
        if let Some(annotation) = inner.annotation.as_deref() {
            sites.push((annotation, inner.into()));
        }
        if let AnyParameterRef::NonVariadic(slot) = param
            && let Some(default) = slot.default.as_deref()
        {
            sites.push((default, slot.into()));
        }
        let mut out = String::new();
        let mut cursor = param.start();
        let mut reshaped = false;
        for (expr, parent) in sites {
            let held = self.source.paren_aware_range(expr.into(), parent);
            let gap = TextRange::new(cursor, held.start());
            if self.source.contains_line_break(gap) {
                return None;
            }
            out.push_str(self.source.slice(gap));
            let landing = Landing {
                column: end_column(&out, indent),
                indent,
                item: param.start(),
            };
            let rest = self.source.slice(TextRange::new(held.end(), param.end()));
            let site_tail = opening_width(rest) + if rest.contains('\n') { 0 } else { tail };
            match self.reshaper.reshaped(expr, held, landing, site_tail) {
                Some(text) => {
                    reshaped = true;
                    out.push_str(&text);
                }
                None => out.push_str(&placed_block(self.source, held, landing)),
            }
            cursor = held.end();
        }
        if !reshaped {
            return None;
        }
        out.push_str(self.source.slice(TextRange::new(cursor, param.end())));
        Some(out)
    }

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

/// The parameter closing `params`, the one no comma follows in the
/// one-per-line form, `None` where a `/` marker closes the list instead.
fn closing_parameter(params: &Parameters) -> Option<TextRange> {
    params
        .kwarg
        .as_deref()
        .map(Ranged::range)
        .or_else(|| params.kwonlyargs.last().map(Ranged::range))
        .or_else(|| params.vararg.as_deref().map(Ranged::range))
        .or_else(|| params.args.last().map(Ranged::range))
}

impl<'a> StatementVisitor<'a> for Layout<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_def(fd);
        }
        walk_stmt(self, stmt);
    }
}

/// Every literal beneath `expr` in source order with the node enclosing
/// it, a literal's own interior left unwalked since `reflow-collections`
/// lays the outer one out before any inside it.
fn literals_beneath<'src>(
    expr: &'src Expr,
    parent: AnyNodeRef<'src>,
) -> Vec<(&'src Expr, AnyNodeRef<'src>)> {
    let mut probe = Literals { found: Vec::new() };
    walk_parented_expr(expr, parent, &mut probe);
    probe.found
}

/// Collects the outermost literals a parented walk reaches.
struct Literals<'src> {
    found: Vec<(&'src Expr, AnyNodeRef<'src>)>,
}

impl<'src> ParentedProbe<'src> for Literals<'src> {
    fn probe(
        &mut self,
        expr: &'src Expr,
        parent: AnyNodeRef<'src>,
        _: &[AnyNodeRef<'src>],
    ) -> Descent {
        if matches!(
            expr,
            Expr::List(_) | Expr::Dict(_) | Expr::Set(_) | Expr::Tuple(_)
        ) {
            self.found.push((expr, parent));
            return Descent::Over;
        }
        Descent::Into
    }
}

/// Every parameter of `params` rendered in source order through
/// `render`, with `/` and bare `*` seated at their canonical positions,
/// `None` where any parameter renders to `None`.
fn rendered_parts<'p>(
    params: &'p Parameters,
    mut render: impl FnMut(AnyParameterRef<'p>) -> Option<Cow<'p, str>>,
) -> Option<Vec<Cow<'p, str>>> {
    let mut parts = Vec::new();
    for param in params.posonlyargs.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if !params.posonlyargs.is_empty() {
        parts.push(Cow::Borrowed("/"));
    }
    for param in params.args.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if let Some(va) = params.vararg.as_deref() {
        parts.push(render(AnyParameterRef::Variadic(va))?);
    } else if !params.kwonlyargs.is_empty() {
        parts.push(Cow::Borrowed("*"));
    }
    for param in params.kwonlyargs.iter().map(AnyParameterRef::NonVariadic) {
        parts.push(render(param)?);
    }
    if let Some(kw) = params.kwarg.as_deref() {
        parts.push(render(AnyParameterRef::Variadic(kw))?);
    }
    Some(parts)
}
