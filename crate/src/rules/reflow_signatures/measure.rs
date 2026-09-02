//! Whether a signature's one-line form fits the budget at the width the
//! later rules settle it to, and the literals along that row a fit test
//! reads.

use ruff_python_ast::{AnyNodeRef, Expr, StmtFunctionDef, helpers::any_over_expr};
use ruff_text_size::{Ranged, TextSize};

use super::terms::Expansion;
use crate::primitives::{
    inline::{display_width, opening_width},
    layout::{is_layoutable, requires_expand},
    padding,
    params::parameter_sites,
    range::return_annotation_range,
    walk::{Descent, ParentedCollector, walk_parented_expr},
};

impl Expansion<'_> {
    /// True when the inline signature `text` sits inside the budget at
    /// the width the later rules settle it to: the padding
    /// `strip-stranded-padding` drops inside each parameter comes off,
    /// and where the row still overflows, the first literal along the
    /// row whose one-row form overflows from its column is the one
    /// `reflow-collections` expands. One inside a parameter leaves that
    /// parameter spanning rows, so the one-line form is out of reach,
    /// whereas one inside the return annotation ends the opening row at
    /// its bracket, which needs a literal that rule expands somewhere
    /// beneath it, leaving a subscript slice holding none to overflow
    /// the row whole.
    pub(super) fn inline_fits(&self, fd: &StmtFunctionDef, text: &str) -> bool {
        let start = fd.parameters.start();
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
                + display_width(&text[..head])
                    .saturating_add_signed(-slack_before(literal.start()));
            let tail = display_width(&text[head + self.source.slice(literal).len()..]);
            if self
                .one_row
                .fitted(self.source, literal, parent, column, tail)
                .is_some()
            {
                continue;
            }
            let in_returns = returns.is_some_and(|ret| ret.range().contains_range(literal.range()));
            let breaks = any_over_expr(literal, &|inner: &Expr| {
                is_layoutable(inner) && requires_expand(inner)
            });
            return in_returns && breaks && column < self.code_line_length;
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
        for param in &fd.parameters {
            let slice = self.source.slice(param.range());
            let Some(found) = text[cursor..].find(slice) else {
                continue;
            };
            let base = cursor + found;
            cursor = base + slice.len();
            for (expr, parent) in parameter_sites(param) {
                for (literal, enclosing) in literals_beneath(expr, parent) {
                    let offset = (literal.start() - param.start()).to_usize();
                    literals.push((literal, enclosing, base + offset));
                }
            }
        }
        if let Some(returns) = fd.returns.as_deref() {
            let annotation = return_annotation_range(returns, fd, self.source);
            // The text closes with the annotation's slice and `:`.
            let base = text.len() - 1 - annotation.len().to_usize();
            for (literal, enclosing) in literals_beneath(returns, fd.into()) {
                let offset = (literal.start() - annotation.start()).to_usize();
                literals.push((literal, enclosing, base + offset));
            }
        }
        literals
    }
}

/// Every literal beneath `expr` in source order with the node enclosing
/// it, a literal's own interior left unwalked since `reflow-collections`
/// lays the outer one out before any inside it.
fn literals_beneath<'src>(
    expr: &'src Expr,
    parent: AnyNodeRef<'src>,
) -> Vec<(&'src Expr, AnyNodeRef<'src>)> {
    let mut probe =
        ParentedCollector::new(Descent::Into, Descent::Over, |expr: &'src Expr, parent| {
            is_layoutable(expr).then_some((expr, parent))
        });
    walk_parented_expr(expr, parent, &mut probe);
    probe.found
}
