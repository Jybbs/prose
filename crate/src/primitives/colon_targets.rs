//! Member constructors for the `:` alignment contexts: dict items,
//! annotated assignments in any scope, annotated function parameters,
//! every Google-style docstring section, and `match` arm cases. Each
//! [`ColonMember`] pairs the pre-colon alignment member with an optional
//! post-colon `value_gap` an aligned or stripped row rewrites to one
//! space. `align_colons` and `align_match_case` consume them to align
//! multi-item groups, whereas `strip_align_padding` consumes them to
//! strip pre-colon padding and normalize the post-colon gap for groups
//! that have no column to align to.

use ruff_python_ast::{
    AnyNodeRef, AnyParameterRef, DictItem, Expr, ExprDict, ExprRef, MatchCase, Parameters, Stmt,
    token::TokenKind,
    visitor::{Visitor as AstVisitor, walk_body, walk_expr, walk_parameters, walk_stmt},
};
use ruff_python_trivia::PythonWhitespace;
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    primitives::{
        aligner,
        docstring::{body_docstring, entry_carrying_sections, unbracketed_colon},
        scope::scoped_body,
    },
    rule::RuleId,
    source::Source,
};

/// Receiver for the colon-context walker. `handle` is the catch-all
/// for annotated assignments, docstring entries, dict entries, and
/// parameters. `match_arms` is split out so a rule can opt out of
/// match-arm alignment by overriding it to a no-op. `rule` names the
/// consuming rule so the group builders can hold its skip-suppressed
/// rows out of alignment. Call `walk` to drive the emitter across
/// `source`'s body.
pub(crate) trait ColonEmitter {
    fn handle(&mut self, members: &[ColonMember]);

    fn match_arms(&mut self, members: &[ColonMember]) {
        self.handle(members);
    }

    fn rule(&self) -> RuleId;

    /// Drives `self` across every `:` context in `source`'s module
    /// body. Recurses into nested classes, functions, matches, and
    /// expressions so a single call covers the whole tree.
    fn walk(&mut self, source: &Source)
    where
        Self: Sized,
    {
        let mut visitor = ContextVisitor {
            emitter: self,
            source,
        };
        visitor.visit_body(&source.ast().body);
    }
}

/// One row in a `:` alignment context, pairing the pre-colon alignment
/// `member` with `value_gap`, the span from just past the colon to the
/// value that an aligned or stripped row rewrites to one space. `None`
/// leaves the post-colon spacing to another rule, as match arms defer to
/// `align_match_case` and docstring entries stay as written.
#[derive(Clone, Copy)]
pub(crate) struct ColonMember {
    pub(crate) member: aligner::Member,
    pub(crate) value_gap: Option<TextRange>,
}

impl ColonMember {
    /// Pairs `member` with no post-colon gap.
    fn bare(member: aligner::Member) -> Self {
        Self {
            member,
            value_gap: None,
        }
    }
}

struct ContextVisitor<'a, E> {
    emitter: &'a mut E,
    source: &'a Source,
}

impl<'a, E: ColonEmitter> AstVisitor<'a> for ContextVisitor<'a, E> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        for group in annotated_assignment_groups(self.source, self.emitter.rule(), body) {
            self.emitter.handle(&group);
        }
        walk_body(self, body);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Dict(d) = expr {
            for group in dict_member_groups(self.source, self.emitter.rule(), d) {
                self.emitter.handle(&group);
            }
        }
        walk_expr(self, expr);
    }

    fn visit_parameters(&mut self, parameters: &'a Parameters) {
        for group in parameter_groups(self.source, self.emitter.rule(), parameters) {
            self.emitter.handle(&group);
        }
        walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Match(m) = stmt {
            self.emitter
                .match_arms(&match_case_members(self.source, &m.cases));
        }
        if let Some((body, _)) = scoped_body(stmt) {
            for group in docstring_sections(self.source, body) {
                self.emitter.handle(&group);
            }
        }
        walk_stmt(self, stmt);
    }
}

/// Builds an alignment member for a `match` arm, anchored on the
/// `:` between the pattern (or its `if` guard) and the arm body's
/// first statement.
pub(crate) fn match_case(source: &Source, case: &MatchCase) -> Option<aligner::Member> {
    let pre_colon_end = match_case_pre_colon_end(case);
    let body_start = case.body.first()?.start();
    aligner::line_anchored_member_between(
        source,
        TextRange::new(case.pattern.start(), pre_colon_end),
        body_start,
        TokenKind::Colon,
    )
}

/// The offset where a `match` arm's pre-colon left-hand side ends, the
/// guard's end when the arm is guarded and the pattern's end otherwise.
pub(crate) fn match_case_pre_colon_end(case: &MatchCase) -> TextSize {
    case.guard
        .as_deref()
        .map_or(case.pattern.end(), Ranged::end)
}

/// Builds a [`ColonMember`] for an annotated assignment, anchored on the
/// `:` between target and annotation. Returns `None` for any other
/// statement shape.
fn annotated_assignment(source: &Source, stmt: &Stmt) -> Option<ColonMember> {
    let ann = stmt.as_ann_assign_stmt()?;
    colon_member(
        source,
        ann.target.range(),
        ann.annotation.as_ref().into(),
        ann.into(),
    )
}

/// Walks `body`, qualifying each statement through `annotated_assignment`,
/// and returns one group per run of contiguous line-adjacent
/// annotated-assignment statements. A single-line row held for `rule`
/// drops out as a transparent hole per [`aligner::line_adjacent_groups`].
fn annotated_assignment_groups(
    source: &Source,
    rule: RuleId,
    body: &[Stmt],
) -> Vec<Vec<ColonMember>> {
    aligner::line_adjacent_groups(source, body, rule, |s| annotated_assignment(source, s))
}

/// Builds a `:`-anchored [`ColonMember`] whose left-hand side is `lhs`,
/// searching for the colon between `lhs.end()` and `value`'s
/// parenthesis-aware start recovered against `parent`. The scan opens
/// past `lhs.end()` so a colon inside the left-hand side (a slice, a
/// nested annotation) never anchors, and the member is rejected when the
/// colon does not share `lhs`'s opening line. `value_gap` runs from just
/// past the colon to that start, the span an aligned or stripped row
/// rewrites to one space, so a wrapping `(` before the value stays put.
fn colon_member(
    source: &Source,
    lhs: TextRange,
    value: ExprRef,
    parent: AnyNodeRef,
) -> Option<ColonMember> {
    let value_start = source.paren_aware_range(value, parent).start();
    let member = aligner::line_anchored_member_between(source, lhs, value_start, TokenKind::Colon)?;
    Some(ColonMember {
        member,
        value_gap: Some(TextRange::new(
            member.gap.end() + TextSize::of(':'),
            value_start,
        )),
    })
}

/// Builds an alignment member for a `key: value` dict entry, anchored
/// on the `:` between key and value. Returns `None` for `**spread`
/// entries that have no key.
fn dict_item(source: &Source, dict: &ExprDict, item: &DictItem) -> Option<ColonMember> {
    let key = item.key.as_ref()?;
    colon_member(source, key.range(), (&item.value).into(), dict.into())
}

/// Returns one group per run of consecutive-line `key: value` entries
/// in `d`. A trailing comment on an entry rides with it and keeps the
/// run going, whereas a standalone comment line or a blank line between
/// two entries closes the active run and starts a fresh one, so each
/// run aligns independently. `**spread` entries skip the colon scan but
/// do not break the run, matching the long-standing rule that an
/// unpacking passes alignment through. A keyed entry whose `:` crosses a
/// line break instead closes the run, so its neighbors do not align a
/// column across the stranded colon.
fn dict_member_groups(source: &Source, rule: RuleId, dict: &ExprDict) -> Vec<Vec<ColonMember>> {
    aligner::adjacent_member_groups(source, &dict.items, false, |item| {
        // A `**spread` (no key) or a skip-held entry joins no group yet
        // bridges the run, so the entries on either side align as one block.
        if item.key.is_none() || aligner::is_held(source, rule, item.start()) {
            return aligner::Slot::Bridge;
        }
        // A keyed entry whose colon sits on a later line carries no
        // single-line anchor, so it breaks the run rather than stranding
        // its colon inside a column its neighbors share.
        dict_item(source, dict, item).map_or(aligner::Slot::Break, aligner::Slot::Member)
    })
}

/// Returns one alignment group per Google-style section in the body's
/// leading docstring, each carrying a member per `name: value` entry
/// anchored on its head line's unbracketed `:`. Returns an empty `Vec`
/// when the body has no leading docstring or carries no entry-bearing
/// section. Each section is its own group, so one section's widths
/// never shift another's column.
fn docstring_sections(source: &Source, body: &[Stmt]) -> Vec<Vec<ColonMember>> {
    let Some(lit) = body_docstring(body) else {
        return Vec::new();
    };
    entry_carrying_sections(source, lit)
        .iter()
        .map(|section| {
            section
                .iter()
                .filter_map(|entry| {
                    let head = source.slice(entry.range).universal_newlines().next()?;
                    let stripped = head.trim_whitespace_start();
                    let colon_rel = unbracketed_colon(stripped)?;
                    let indent_len = head.len() - stripped.len();
                    let colon_start =
                        entry.range.start() + TextSize::of(&head[..indent_len + colon_rel]);
                    Some(ColonMember::bare(aligner::line_anchored_member(
                        source,
                        colon_start,
                    )))
                })
                .collect()
        })
        .collect()
}

/// Returns one alignment member per `case` arm in `cases`.
fn match_case_members(source: &Source, cases: &[MatchCase]) -> Vec<ColonMember> {
    cases
        .iter()
        .filter_map(|c| match_case(source, c))
        .map(ColonMember::bare)
        .collect()
}

/// Builds an alignment member for an annotated function parameter,
/// anchored on the `:` between name and annotation. Returns `None` for
/// unannotated parameters, signaling a group break to callers.
fn parameter(source: &Source, param: AnyParameterRef<'_>) -> Option<ColonMember> {
    let annotation = param.annotation()?;
    colon_member(
        source,
        param.name().range(),
        annotation.into(),
        param.as_parameter().into(),
    )
}

/// Walks `params` in source order and returns one group per run of
/// contiguous annotated parameters, splitting at every unannotated
/// parameter. A parameter skip-suppressed for `rule` drops out of its
/// group as a transparent hole, leaving its neighbors to align.
fn parameter_groups(source: &Source, rule: RuleId, params: &Parameters) -> Vec<Vec<ColonMember>> {
    aligner::parameter_split_groups(params, |p| parameter(source, p))
        .into_iter()
        .map(|group| aligner::retain_unheld(source, rule, group, |m| m.member.line_start))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::parse;

    #[test]
    fn annotated_assignment_rejects_cross_line_colon() {
        // The target ends on its own line and the annotation `:` opens
        // the next.
        let source = parse("class C:\n    x \\\n        : int\n");
        let class = source.ast().body[0].as_class_def_stmt().expect("class");
        assert!(annotated_assignment(&source, &class.body[0]).is_none());
    }

    #[test]
    fn dict_item_rejects_cross_line_key() {
        // The key ends on its own line and the `:` opens the next, so the
        // entry yields no alignable member.
        let source = parse("d = {\n    k\n    : v,\n}\n");
        let dict = source.ast().body[0]
            .as_assign_stmt()
            .expect("assign")
            .value
            .as_dict_expr()
            .expect("dict");
        assert!(dict_item(&source, dict, &dict.items[0]).is_none());
    }

    #[test]
    fn match_case_rejects_multiline_pattern() {
        // The pattern spans several lines, placing the `:` off the line
        // where the pattern opens.
        let source = parse("match x:\n    case (\n        1,\n        2,\n    ):\n        y\n");
        let m = source.ast().body[0].as_match_stmt().expect("match");
        assert!(match_case(&source, &m.cases[0]).is_none());
    }

    #[test]
    fn parameter_rejects_cross_line_colon() {
        // The parameter name ends on its own line and the annotation `:`
        // opens the next.
        let source = parse("def f(\n    a\n    : int,\n):\n    pass\n");
        let func = source.ast().body[0].as_function_def_stmt().expect("def");
        let param = func.parameters.iter_source_order().next().expect("param");
        assert!(parameter(&source, param).is_none());
    }
}
