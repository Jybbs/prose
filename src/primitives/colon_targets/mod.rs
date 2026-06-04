//! Member constructors for the five `:` alignment contexts. The
//! contexts are dict items, Pydantic-style class fields, annotated
//! function parameters, Google/numpy docstring `Args:` entries, and
//! `match` arm cases. `align_colons` and `align_match_case` consume
//! them to align multi-item groups, whereas `strip_align_padding` consumes
//! them to strip pre-colon padding from groups that have no column to
//! align to.

use ruff_python_ast::{
    AnyParameterRef, DictItem, Expr, MatchCase, Parameters, Stmt,
    token::TokenKind,
    visitor::{Visitor as AstVisitor, walk_expr, walk_parameters, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{primitives::aligner, rule::RuleId, source::Source};

mod docstring;

use docstring::docstring_args;

/// Receiver for the colon-context walker. `handle` is the catch-all
/// for class fields, docstring args, dict entries, and parameters.
/// `match_arms` is split out so a rule can opt out of match-arm
/// alignment by overriding it to a no-op. `rule` names the consuming
/// rule so the group builders can hold its skip-suppressed rows out of
/// alignment. Call `walk` to drive the emitter across `source`'s body.
pub(crate) trait ColonEmitter {
    fn handle(&mut self, _members: &[aligner::Member]) {}

    fn match_arms(&mut self, members: &[aligner::Member]) {
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

struct ContextVisitor<'a, E> {
    emitter: &'a mut E,
    source: &'a Source,
}

impl<'a, E: ColonEmitter> AstVisitor<'a> for ContextVisitor<'a, E> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Dict(d) = expr {
            for group in dict_member_groups(self.source, self.emitter.rule(), &d.items) {
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
        match stmt {
            Stmt::ClassDef(cd) => {
                for group in class_field_groups(self.source, self.emitter.rule(), &cd.body) {
                    self.emitter.handle(&group);
                }
                self.emitter.handle(&docstring_args(self.source, &cd.body));
            }
            Stmt::FunctionDef(fd) => {
                self.emitter.handle(&docstring_args(self.source, &fd.body));
            }
            Stmt::Match(m) => {
                self.emitter
                    .match_arms(&match_case_members(self.source, &m.cases));
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Builds an alignment member for a `match` arm, anchored on the
/// `:` between the pattern (or its `if` guard) and the arm body's
/// first statement.
pub(crate) fn match_case(source: &Source, case: &MatchCase) -> Option<aligner::Member> {
    let pre_colon_end = case
        .guard
        .as_deref()
        .map_or(case.pattern.end(), Ranged::end);
    let body_start = case.body.first()?.start();
    colon_member(source, pre_colon_end, body_start)
}

/// Builds an alignment member for a class-body annotated assignment,
/// anchored on the `:` between target and annotation. Returns `None`
/// for any other statement shape.
fn class_field(source: &Source, stmt: &Stmt) -> Option<aligner::Member> {
    let ann = stmt.as_ann_assign_stmt()?;
    colon_member(source, ann.target.end(), ann.annotation.start())
}

/// Walks `body`, qualifying each statement through `class_field`,
/// and returns one group per run of contiguous line-adjacent
/// annotated-assignment statements. A field skip-suppressed for `rule`
/// drops out as a transparent hole per [`aligner::line_adjacent_groups`].
fn class_field_groups(source: &Source, rule: RuleId, body: &[Stmt]) -> Vec<Vec<aligner::Member>> {
    aligner::line_adjacent_groups(source, body, rule, |s| class_field(source, s))
}

/// Builds a `:`-anchored alignment member from the half-open span
/// `[start, end)` searched for the colon token.
fn colon_member(source: &Source, start: TextSize, end: TextSize) -> Option<aligner::Member> {
    aligner::line_anchored_member_at_kind(source, TextRange::new(start, end), TokenKind::Colon)
}

/// Builds an alignment member for a `key: value` dict entry, anchored
/// on the `:` between key and value. Returns `None` for `**spread`
/// entries that have no key.
fn dict_item(source: &Source, item: &DictItem) -> Option<aligner::Member> {
    let key = item.key.as_ref()?;
    colon_member(source, key.end(), item.value.start())
}

/// Returns one group per run of consecutive-line `key: value` entries
/// in `d`. A trailing comment on an entry rides with it and keeps the
/// run going, whereas a standalone comment line or a blank line between
/// two entries closes the active run and starts a fresh one, so each
/// run aligns independently. `**spread` entries skip the colon scan but
/// do not break the run, matching the long-standing rule that an
/// unpacking passes alignment through.
fn dict_member_groups(
    source: &Source,
    rule: RuleId,
    items: &[DictItem],
) -> Vec<Vec<aligner::Member>> {
    let mut groups: Vec<Vec<aligner::Member>> = Vec::new();
    let mut current: Vec<aligner::Member> = Vec::new();
    let mut prev_end: Option<TextSize> = None;
    for item in items {
        let member = match dict_item(source, item) {
            Some(member) if !aligner::is_held(source, rule, item.start()) => member,
            _ => {
                // A `**spread` (no key) or a skip-held entry joins no
                // group yet bridges the run, extending the anchor so the
                // entries on either side still align as one block.
                if let Some(end) = prev_end.as_mut() {
                    *end = item.end();
                }
                continue;
            }
        };
        let extends = prev_end.is_some_and(|end| source.consecutive_lines(end, item.start()));
        if !extends {
            aligner::flush_run(&mut groups, &mut current);
        }
        current.push(member);
        prev_end = Some(item.end());
    }
    aligner::flush_run(&mut groups, &mut current);
    groups
}

/// Returns one alignment member per `case` arm in `cases`.
fn match_case_members(source: &Source, cases: &[MatchCase]) -> Vec<aligner::Member> {
    cases.iter().filter_map(|c| match_case(source, c)).collect()
}

/// Builds an alignment member for an annotated function parameter,
/// anchored on the `:` between name and annotation. Returns `None` for
/// unannotated parameters, signaling a group break to callers.
fn parameter(source: &Source, param: AnyParameterRef<'_>) -> Option<aligner::Member> {
    let annotation = param.annotation()?;
    colon_member(source, param.name().end(), annotation.start())
}

/// Walks `params` in source order and returns one group per run of
/// contiguous annotated parameters, splitting at every unannotated
/// parameter. A parameter skip-suppressed for `rule` drops out of its
/// group as a transparent hole, leaving its neighbors to align.
fn parameter_groups(
    source: &Source,
    rule: RuleId,
    params: &Parameters,
) -> Vec<Vec<aligner::Member>> {
    aligner::parameter_split_groups(params, |p| parameter(source, p))
        .into_iter()
        .map(|group| {
            group
                .into_iter()
                .filter(|m| !aligner::is_held(source, rule, m.line_start))
                .collect()
        })
        .collect()
}
