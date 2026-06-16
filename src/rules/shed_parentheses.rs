//! Sheds a redundant grouping parenthesis pair, the pair whose removal
//! leaves the parse unchanged. Each candidate is reparsed with the pair
//! stripped and kept where the bare form fails to parse or shifts the
//! tree, so a precedence pair, a generator, a walrus binding, and a
//! one-element tuple all survive. A wrapped pair folds onto one line
//! when the bare form fits the budget, and a pair nested inside another
//! redundant pair sheds in the same pass.

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Arguments, Expr, Stmt,
    comparable::ComparableStmt,
    token::parenthesized_range,
    visitor::{Visitor, walk_arguments, walk_expr, walk_stmt},
};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        edit::{singleton_groups, splice_reparse},
        inline::{single_line_form, whitespace_runs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ShedParentheses {
    code_line_length: usize,
}

impl ShedParentheses {
    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
        }
    }
}

impl Rule for ShedParentheses {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut walker = Walker {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            in_collapse: false,
            parents: vec![AnyNodeRef::from(source.ast())],
            source,
        };
        walker.visit_body(&source.ast().body);
        singleton_groups(walker.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

struct Walker<'a> {
    code_line_length: usize,
    edits: Vec<Edit>,
    in_collapse: bool,
    parents: Vec<AnyNodeRef<'a>>,
    source: &'a Source,
}

impl Walker<'_> {
    /// Reports whether splicing the bare interior in place of `pair`
    /// reparses to the same statement tree, the question that decides
    /// whether the pair carries syntax or only wraps.
    fn preserves_tree(&self, pair: TextRange, bare: &str) -> bool {
        let Ok(reparsed) = splice_reparse(
            self.source,
            self.source.module_range(),
            pair,
            bare,
            parse_module,
        ) else {
            return false;
        };
        self.source
            .ast()
            .body
            .iter()
            .map(ComparableStmt::from)
            .eq(reparsed.syntax().body.iter().map(ComparableStmt::from))
    }

    /// Emits an edit folding each line-spanning whitespace run inside
    /// `inner` to a single space, the join a multi-line interior needs
    /// before its parentheses can go. A run sitting inside a nested pair
    /// stays whole here so the nested pair's own deletions do not collide.
    fn push_fold_edits(&mut self, inner: TextRange) {
        let text = self.source.slice(inner);
        for (begin, len) in whitespace_runs(text) {
            if text[begin..begin + len].contains('\n') {
                let start = inner.start() + TextSize::try_from(begin).expect("offset fits u32");
                let end = start + TextSize::try_from(len).expect("run length fits u32");
                self.edits.push(Edit::range_replacement(
                    " ".to_owned(),
                    TextRange::new(start, end),
                ));
            }
        }
    }

    /// Emits the edits shedding `expr`'s grouping pair when one encloses
    /// it and removal leaves the parse unchanged. Returns whether this
    /// pair opened a fold the caller propagates to `expr`'s children, so
    /// each nested pair drops only its parentheses and the fold runs once.
    fn shed(&mut self, expr: &Expr, parent: AnyNodeRef) -> bool {
        let Some(pair) = parenthesized_range(expr.into(), parent, self.source.tokens()) else {
            return false;
        };
        // A walrus binding keeps its pair whatever the context, since the
        // grammar needs it almost everywhere, and a multi-line return
        // annotation is signature-layout's to reshape, so neither sheds here.
        if expr.is_named_expr()
            || (is_return_annotation(expr, parent) && self.source.contains_line_break(pair))
            || self.source.intersects_comment(pair)
        {
            return false;
        }
        let inner = expr.range();
        let Some(bare) = single_line_form(expr, self.source.slice(inner)) else {
            return false;
        };
        let pair_wraps = self.source.contains_line_break(pair);
        if !self.in_collapse
            && pair_wraps
            && self.source.column_of(pair.start()) + bare.width() > self.code_line_length
        {
            return false;
        }
        if !self.preserves_tree(pair, &bare) {
            return false;
        }
        let (open, close) = if self.in_collapse {
            let paren = TextSize::new(1);
            (
                TextRange::at(pair.start(), paren),
                TextRange::at(pair.end() - paren, paren),
            )
        } else {
            (
                TextRange::new(pair.start(), inner.start()),
                TextRange::new(inner.end(), pair.end()),
            )
        };
        self.edits.push(Edit::range_deletion(open));
        self.edits.push(Edit::range_deletion(close));
        if !self.in_collapse && pair_wraps {
            self.push_fold_edits(inner);
            return true;
        }
        false
    }
}

impl<'a> Visitor<'a> for Walker<'a> {
    fn visit_arguments(&mut self, arguments: &'a Arguments) {
        self.parents.push(arguments.into());
        walk_arguments(self, arguments);
        self.parents.pop();
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let parent = *self.parents.last().expect("seeded with the module node");
        let opened_fold = self.shed(expr, parent);
        let outer_collapse = self.in_collapse;
        self.in_collapse |= opened_fold;
        self.parents.push(expr.into());
        walk_expr(self, expr);
        self.parents.pop();
        self.in_collapse = outer_collapse;
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.parents.push(stmt.into());
        walk_stmt(self, stmt);
        self.parents.pop();
    }
}

/// True when `expr` is the return annotation of the function `parent`.
fn is_return_annotation(expr: &Expr, parent: AnyNodeRef) -> bool {
    matches!(
        parent,
        AnyNodeRef::StmtFunctionDef(fd)
            if fd.returns.as_deref().is_some_and(|ann| ann.range() == expr.range())
    )
}
