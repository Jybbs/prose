//! Sheds a redundant grouping parenthesis pair, the pair whose removal
//! leaves the parse unchanged. Each candidate is reparsed with the pair
//! stripped and kept where the bare form fails to parse or shifts the
//! tree, so a precedence pair, a generator, a walrus binding, and a
//! one-element tuple all survive. A wrapped pair folds onto one line
//! when the bare form fits the budget, measured from the column the pair
//! reaches once the pass's earlier edits apply and narrowed by the pairs
//! nested inside it, which shed in the same pass.

use std::{borrow::Cow, cmp::Reverse};

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, comparable::ComparableStmt, token::parenthesized_range};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        edit::{apply_inline_edits, insert_edit, singleton_groups, splice_reparse},
        inline::{end_column, single_line_form, soft_wrap_runs},
        walk::{Descent, ParentedProbe, walk_parented_exprs},
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
        let mut scout = Scout {
            candidates: Vec::new(),
            source,
        };
        walk_parented_exprs(source.ast(), &mut scout);
        let mut candidates = scout.candidates;
        candidates.sort_unstable_by_key(|c| (c.pair.start(), Reverse(c.pair.end())));
        let mut shedder = Shedder {
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            folds: Vec::new(),
            source,
        };
        shedder.shed(&candidates);
        singleton_groups(shedder.edits)
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// One grouping pair whose removal leaves the parse unchanged, carrying
/// its interior range and that interior's single-line form.
struct Candidate<'src> {
    bare: Cow<'src, str>,
    inner: TextRange,
    pair: TextRange,
}

/// Collects every structurally redundant pair in a module, leaving the
/// budget to [`Shedder`].
struct Scout<'a> {
    candidates: Vec<Candidate<'a>>,
    source: &'a Source,
}

impl<'a> Scout<'a> {
    /// The candidate `expr` contributes, or `None` where no pair
    /// encloses it, the pair carries syntax, its interior has no
    /// single-line form, or stripping the pair shifts the parse.
    fn candidate(&self, expr: &'a Expr, parent: AnyNodeRef) -> Option<Candidate<'a>> {
        let pair = parenthesized_range(expr.into(), parent, self.source.tokens())?;
        // A walrus binding keeps its pair whatever the context, since the
        // grammar needs it almost everywhere, and a multi-line return
        // annotation is signature-layout's to reshape, so neither sheds here.
        if expr.is_named_expr()
            || (is_return_annotation(expr, parent) && self.source.contains_line_break(pair))
            || self.source.intersects_comment(pair)
        {
            return None;
        }
        let inner = expr.range();
        let bare = single_line_form(expr, self.source.slice(inner))?;
        self.preserves_tree(pair, &bare)
            .then_some(Candidate { bare, inner, pair })
    }

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
}

impl<'a> ParentedProbe<'a> for Scout<'a> {
    fn probe(&mut self, expr: &'a Expr, parent: AnyNodeRef<'a>) -> Descent {
        self.candidates.extend(self.candidate(expr, parent));
        Descent::Into
    }
}

/// Turns a candidate list into edits, walking it in source order so each
/// budget test reads the columns the preceding edits produce.
struct Shedder<'a> {
    code_line_length: usize,
    edits: Vec<Edit>,
    folds: Vec<TextRange>,
    source: &'a Source,
}

impl Shedder<'_> {
    /// True when joining `candidate` leaves its line inside the budget,
    /// the interior narrowed by the two columns each nested candidate
    /// sheds alongside it.
    fn fits(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let nested = candidates
            .iter()
            .filter(|other| candidate.inner.contains_range(other.pair))
            .count();
        let width = candidate.bare.width() - 2 * nested;
        self.shifted_column(candidate.pair.start()) + width <= self.code_line_length
    }

    /// Emits an edit folding each line-spanning whitespace run inside
    /// `inner` to a single space, the join a multi-line interior needs
    /// before its parentheses can go.
    fn push_fold_edits(&mut self, inner: TextRange) {
        let text = self.source.slice(inner);
        for (begin, len) in soft_wrap_runs(text) {
            let start = inner.start() + TextSize::try_from(begin).expect("offset fits u32");
            let end = start + TextSize::try_from(len).expect("run length fits u32");
            insert_edit(
                &mut self.edits,
                Edit::range_replacement(" ".to_owned(), TextRange::new(start, end)),
            );
        }
    }

    /// Emits the deletions for every candidate, folding a wrapped pair
    /// whose joined line fits the budget. A candidate inside an open fold
    /// drops its parentheses alone, leaving that fold's own edits to
    /// close the break.
    fn shed(&mut self, candidates: &[Candidate]) {
        for candidate in candidates {
            let Candidate { inner, pair, .. } = *candidate;
            self.folds.retain(|fold| fold.contains_range(pair));
            let collapsing = !self.folds.is_empty();
            let folding = !collapsing && self.source.contains_line_break(pair);
            if folding && !self.fits(candidate, candidates) {
                continue;
            }
            let (open, close) = if collapsing {
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
            insert_edit(&mut self.edits, Edit::range_deletion(open));
            insert_edit(&mut self.edits, Edit::range_deletion(close));
            if folding {
                self.push_fold_edits(inner);
                self.folds.push(pair);
            }
        }
    }

    /// The column `offset` reaches once the edits emitted so far apply.
    fn shifted_column(&self, offset: TextSize) -> usize {
        end_column(
            &apply_inline_edits(self.source, TextRange::up_to(offset), &self.edits),
            0,
        )
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
