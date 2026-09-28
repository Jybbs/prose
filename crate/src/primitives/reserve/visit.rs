//! Walks a module collecting the columns each reserved run settles to.

use ruff_source_file::LineRanges;
use rustc_hash::FxHashMap;

use super::*;
use crate::{
    primitives::{binding::assigned_value, colon_targets, range::overlaps},
    rules::align_colons::AlignColons,
};

/// Collects the rule's runs and the value each member's row carries,
/// keyed by the paren-aware start the member's value gap ends at, a
/// statement whose value `measure` joins or breaks open extending its
/// run.
pub(super) struct ReserveVisitor<'a> {
    /// The annotated-assignment runs `align-colons` forms over the same
    /// rows as the statement runs, empty where that rule is off.
    pub(super) colons: Vec<Run>,
    pub(super) measure: &'a Measure<'a>,
    /// The completion of a carried table, the walk forming a body's
    /// runs over the slices it names and descending into the statements
    /// its windows reach alone, or `None` for a walk over the whole
    /// tree.
    pub(super) reform: Option<&'a Reform>,
    pub(super) rule: RuleId,
    pub(super) runs: Vec<Run>,
    /// The owner and row span of each carried run formed over a reorder
    /// rule's seating, empty for a walk over the whole tree.
    pub(super) seated: &'a [(TextRange, TextRange)],
    pub(super) source: &'a Source,
    /// The statement the walk is inside, the scope a keyword or
    /// parameter run forms over, the module ahead of any.
    pub(super) stmt: TextRange,
    pub(super) stranding: Stranding,
    pub(super) values: FxHashMap<TextSize, (&'a Expr, AnyNodeRef<'a>)>,
    /// The owner and range of each body whose statement runs a completion
    /// formed afresh. The table drops every carried run over one of these
    /// bodies.
    pub(super) whole: Vec<(TextRange, TextRange)>,
}

impl<'a> ReserveVisitor<'a> {
    /// Records `value` under the start of its paren-aware range against
    /// `parent`.
    fn note(&mut self, value: &'a Expr, parent: AnyNodeRef<'a>) {
        let start = self.source.paren_aware_range(value.into(), parent).start();
        self.values.insert(start, (value, parent));
    }

    /// Notes the value `assigned_value` reads off each statement of
    /// `body` against that statement.
    fn note_values(&mut self, body: &'a [Stmt]) {
        for stmt in body {
            if let Some(value) = assigned_value(stmt) {
                self.note(value, stmt.into());
            }
        }
    }

    /// Records the statement runs of `body`, owned by `owner`, and the
    /// annotated-assignment runs `align-colons` forms over the same rows
    /// where that rule is on, then notes each statement's value. Each
    /// statement reads in `seating` where a reorder rule gives `body` one,
    /// and one whose value the visitor's measure joins or breaks open
    /// extends its run.
    fn record_statements(
        &mut self,
        body: &'a [Stmt],
        owner: TextRange,
        seating: Option<&[(usize, bool)]>,
    ) {
        let measure = self.measure;
        let joins = |stmt: &'a Stmt| measure.joins(stmt);
        let rows: Vec<(&'a Stmt, bool)> = seating.map_or_else(
            || self.source.adjacent_rows(body).collect(),
            |rows| {
                rows.iter()
                    .map(|&(slot, below)| (&body[slot], below))
                    .collect()
            },
        );
        let seated = seating.is_some();
        if measure.colons.is_some() {
            let groups = colon_targets::annotated_assignment_groups(
                self.source,
                AlignColons::SLUG,
                rows.iter().copied(),
                self.stranding,
                joins,
            );
            self.colons.extend(runs(groups, true, owner, true, seated));
        }
        let groups = equal_targets::assignment_groups(
            self.source,
            self.rule,
            rows.iter().copied(),
            self.stranding,
            joins,
        );
        self.runs.extend(runs(groups, false, owner, true, seated));
        self.note_values(body);
    }
}

impl<'a> Visitor<'a> for ReserveVisitor<'a> {
    /// Forms `body`'s statement runs over the whole body, or over the
    /// slices a completion names, and descends into every statement or
    /// into those its windows reach. A completion forms afresh every run
    /// of a body a reorder rule seats now or seated when the table was
    /// carried.
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let owner = self.stmt;
        let Some(range) = body.first().zip(body.last()).map(|(first, last)| {
            TextRange::new(self.source.text().line_start(first.start()), last.end())
        }) else {
            return;
        };
        let seating = self.measure.seating(body);
        let seated = seating.is_some()
            || self
                .seated
                .iter()
                .any(|&(stmt, span)| stmt == owner && range.contains_range(span));
        match self.reform {
            Some(reform) if !seated => {
                for slice in reform.slices(owner, body) {
                    self.record_statements(&body[slice], owner, None);
                }
            }
            reform => {
                if reform.is_some() {
                    self.whole.push((owner, range));
                }
                self.record_statements(body, owner, seating);
            }
        }
        for stmt in body {
            if self
                .reform
                .is_none_or(|reform| overlaps(stmt.range(), &reform.windows))
            {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.runs.extend(runs(
                equal_targets::keyword_groups(self.source, self.rule, call, true, self.stranding),
                false,
                self.stmt,
                false,
                false,
            ));
            for keyword in &call.arguments.keywords {
                self.note(&keyword.value, keyword.into());
            }
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let outer = std::mem::replace(&mut self.stmt, stmt.range());
        if let Stmt::FunctionDef(def) = stmt {
            self.runs.extend(runs(
                equal_targets::parameter_groups(
                    self.source,
                    self.rule,
                    &def.parameters,
                    self.stranding,
                ),
                true,
                stmt.range(),
                false,
                false,
            ));
            for param in def.parameters.iter_non_variadic_params() {
                if let Some(default) = param.default.as_deref() {
                    self.note(default, param.into());
                }
            }
        }
        walk::walk_stmt(self, stmt);
        self.stmt = outer;
    }
}

/// One collected alignment run, `candidate` true where the rule aligns
/// it to a column or leaves it alone and false where it buffers each
/// row, and `scope` the statement the run forms inside, the one whose
/// body holds a statement run or whose expressions hold a keyword or
/// parameter run, the module itself for a module-body run.
pub(super) struct Run {
    /// True for a run formed over a body's statements, false for one
    /// formed over a statement's keywords or parameters.
    pub(super) body: bool,
    pub(super) candidate: bool,
    pub(super) members: Vec<aligner::Member>,
    pub(super) scope: TextRange,
    /// True for a run formed over a body a reorder rule seats other than
    /// as written.
    pub(super) seated: bool,
}

/// Builds one run per group of `groups`, each formed inside `scope`.
fn runs(
    groups: Vec<Vec<aligner::Member>>,
    candidate: bool,
    scope: TextRange,
    body: bool,
    seated: bool,
) -> impl Iterator<Item = Run> {
    groups.into_iter().map(move |members| Run {
        body,
        candidate,
        members,
        scope,
        seated,
    })
}
