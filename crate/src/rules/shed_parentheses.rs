//! Sheds a redundant grouping parenthesis pair, the pair whose removal
//! leaves the parse unchanged. Each candidate is reparsed with the pair
//! stripped and kept where the bare form fails to parse or shifts the
//! tree, so a precedence pair, a generator, a walrus binding, and a
//! one-element tuple all survive. A wrapped pair folds onto one line
//! when the bare form fits the budget, measured from the column the pair
//! reaches once the pass's earlier edits apply and narrowed by the pairs
//! nested inside it, which shed in the same pass. A wrapped pair whose
//! breaks a bracket inside it holds sheds in place, leaving the rows
//! inside to the layout rules.

use std::{borrow::Cow, cmp::Reverse};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    AnyNodeRef, Expr,
    token::{TokenKind, parenthesized_range},
};
use ruff_python_trivia::PythonWhitespace;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        INDENT_STEP,
        edit::{apply_inline_edits, insert_edit, singleton_groups},
        inline::{end_column, folded_line_form, indent_width, soft_wrap_runs},
        splice::splice_preserves_tree,
        tokens::{is_closer, is_opener},
        travel::frozen_rows,
        walk::{Descent, filter_map_over_parented_exprs},
    },
    rule::{Rule, RuleId},
    source::Source,
};

pub(crate) struct ShedParentheses {
    code_line_length: usize,
}

impl ShedParentheses {
    pub(crate) const MESSAGE: &'static str = "shed a redundant grouping parenthesis pair";

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
        }
    }
}

impl Rule for ShedParentheses {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut candidates =
            filter_map_over_parented_exprs(source.ast(), Descent::Into, |expr, parent| {
                candidate(source, expr, parent)
            });
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
/// its interior range and that interior's single-line form, `None`
/// where only a bracket inside the interior holds its breaks.
struct Candidate<'src> {
    bare: Option<Cow<'src, str>>,
    inner: TextRange,
    pair: TextRange,
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
    /// sheds alongside it, and false for an interior no fold joins.
    fn fits(&self, candidate: &Candidate, candidates: &[Candidate]) -> bool {
        let Some(bare) = &candidate.bare else {
            return false;
        };
        let nested = candidates
            .iter()
            .filter(|other| candidate.inner.contains_range(other.pair))
            .count();
        let width = bare.width() - 2 * nested;
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
    /// close the break. A wrapped pair whose joined line overflows sheds
    /// in place when an enclosing bracket holds its breaks, and holds
    /// otherwise.
    fn shed(&mut self, candidates: &[Candidate]) {
        for candidate in candidates {
            let Candidate { inner, pair, .. } = *candidate;
            self.folds.retain(|fold| fold.contains_range(pair));
            let collapsing = !self.folds.is_empty();
            let mut folding = !collapsing && self.source.contains_line_break(pair);
            let (open, close) = if collapsing {
                let paren = TextSize::new(1);
                (
                    TextRange::at(pair.start(), paren),
                    TextRange::at(pair.end() - paren, paren),
                )
            } else if folding && !self.fits(candidate, candidates) {
                let Some((open, close)) = self.shed_in_place_spans(candidate) else {
                    continue;
                };
                folding = false;
                self.push_reseat_edits(pair, open, TextRange::new(open.end(), close.start()));
                (open, close)
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

    /// Emits the edits re-seating the continuation rows of `interior`,
    /// the text between the shed parens of `pair`, by the columns the
    /// opener took where the interior opens on the opener's row: a row
    /// indented to the column directly past the opener tracks that
    /// delimiter and follows it leftward whatever else shares its
    /// column, a row hanging one or two indent steps past the opener's
    /// row otherwise keeps its column, and any other row indented to
    /// the column of a token on the opening row follows the token
    /// leftward. A row inside a row-spanning string keeps its column.
    fn push_reseat_edits(&mut self, pair: TextRange, open: TextRange, interior: TextRange) {
        let block = self.source.slice(interior);
        // An opener ending its row leaves nothing on that row to align to,
        // so the rows below it hang by construction.
        if !self.source.same_line(pair.start(), interior.start()) || block.starts_with(['\r', '\n'])
        {
            return;
        }
        let shift = self.source.slice(open).width();
        let row_indent = self.source.line_indent_width(pair.start());
        let delimiter_aligned = self.source.column_of(interior.start());
        let hangs = |indent: usize| {
            indent == row_indent + INDENT_STEP || indent == row_indent + 2 * INDENT_STEP
        };
        let opening_row = TextRange::new(
            interior.start(),
            self.source.row_tail(interior.start()).end(),
        );
        let anchors: Vec<usize> = self
            .source
            .tokens_overlapping(opening_row)
            .filter(|token| opening_row.contains(token.start()))
            .map(|token| self.source.column_of(token.start()))
            .collect();
        let frozen = frozen_rows(self.source, interior);
        let mut row_start = interior.start();
        for (row, line) in block.split_inclusive('\n').enumerate() {
            let indent = indent_width(line);
            let follows =
                indent == delimiter_aligned || (!hangs(indent) && anchors.contains(&indent));
            if row > 0 && frozen.get(row) != Some(&true) && follows {
                // An indent-only deletion at the row start composes with a
                // shed nested inside the row, whose own deletions sit at its
                // parens.
                let taken = TextSize::try_from(shift.min(indent)).expect("indent fits u32");
                insert_edit(
                    &mut self.edits,
                    Edit::range_deletion(TextRange::at(row_start, taken)),
                );
            }
            row_start += line.text_len();
        }
    }

    /// The deletion spans shedding `candidate` in place, leaving its
    /// breaks where the source wrote them: the opening paren with the
    /// horizontal whitespace around it up to a break on either side, and
    /// the span from the interior's end through the closing paren. `None`
    /// where the splice does not preserve the statement tree, the shape a
    /// pair outside any enclosing bracket takes once its boundary break
    /// loses the paren that licensed it.
    fn shed_in_place_spans(&self, candidate: &Candidate) -> Option<(TextRange, TextRange)> {
        let Candidate { inner, pair, .. } = *candidate;
        let text = self.source.text();
        let after = &text[pair.start().to_usize() + 1..];
        let trailing = after.text_len() - after.trim_whitespace_start().text_len();
        let mut open = TextRange::at(pair.start(), TextSize::of('(') + trailing);
        // The paren gone, whitespace ahead of it would trail its row, so
        // a break directly past the span pulls that run into the span.
        if text[open.end().to_usize()..].starts_with(['\r', '\n']) {
            let before = &text[..pair.start().to_usize()];
            let leading = before.text_len() - before.trim_whitespace_end().text_len();
            open = TextRange::new(open.start() - leading, open.end());
        }
        let close = TextRange::new(inner.end(), pair.end());
        let bare = self.source.slice(TextRange::new(open.end(), close.start()));
        splice_preserves_tree(self.source, pair, bare).then_some((open, close))
    }

    /// The column `offset` reaches once the edits emitted so far apply.
    /// Measuring from the enclosing logical line rather than the file
    /// start reads the same row, since a fold never joins across the
    /// `Newline` token that opens the line.
    fn shifted_column(&self, offset: TextSize) -> usize {
        let head = self.source.logical_line_start(offset);
        end_column(&apply_inline_edits(self.source, head, &self.edits), 0)
    }
}

/// The candidate `expr` contributes, or `None` where no pair encloses
/// it, the pair carries syntax, or stripping the pair shifts the parse.
/// An interior no fold joins still qualifies where the brackets inside
/// it hold its breaks.
fn candidate<'src>(
    source: &'src Source,
    expr: &'src Expr,
    parent: AnyNodeRef,
) -> Option<Candidate<'src>> {
    let pair = parenthesized_range(expr.into(), parent, source.tokens())?;
    // A walrus binding keeps its pair whatever the context, since the
    // grammar needs it almost everywhere, and a multi-line return
    // annotation belongs to `reflow-signatures`, so neither sheds here.
    if expr.is_named_expr()
        || (is_return_annotation(expr, parent) && source.contains_line_break(pair))
        || source.intersects_comment(pair)
    {
        return None;
    }
    let inner = expr.range();
    let bare = folded_line_form(expr, source.slice(inner));
    if bare.is_none() && !breaks_held_inside(source, inner) {
        return None;
    }
    let probe = bare.as_deref().unwrap_or_else(|| source.slice(inner));
    splice_preserves_tree(source, pair, probe).then_some(Candidate { bare, inner, pair })
}

/// True when every line break inside `inner` sits inside a bracket
/// `inner` itself opens, so the pair around it holds none of them.
fn breaks_held_inside(source: &Source, inner: TextRange) -> bool {
    let mut depth = 0_usize;
    source
        .tokens_overlapping(inner)
        .filter(|token| inner.contains(token.start()))
        .all(|token| {
            if is_opener(token.kind()) {
                depth += 1;
            } else if is_closer(token.kind()) {
                depth -= 1;
            }
            depth > 0 || token.kind() != TokenKind::NonLogicalNewline
        })
}

/// True when `expr` is the return annotation of the function `parent`.
fn is_return_annotation(expr: &Expr, parent: AnyNodeRef) -> bool {
    matches!(
        parent,
        AnyNodeRef::StmtFunctionDef(fd)
            if fd.returns.as_deref().is_some_and(|ann| ann.range() == expr.range())
    )
}
