//! Flags each physical line still over its cap, `import_line_length`
//! inside an import statement and `code_line_length` elsewhere, once no
//! layout rule can shorten it. A line holding a construct a layout rule
//! splits is left to that rule, unless a skip holds that rule there or
//! its code fits and only a trailing comment runs past the cap, and a
//! line whose code fits short of a trailing pragma or `prose` directive
//! is not flagged. A line whose overflow sits inside one string literal
//! holding interior whitespace carries the [`split`] form as a
//! display-only suggestion, gated by `suggest_string_splits`. Lint-only.

use ruff_python_ast::{
    Expr, ExprStringLiteral, InterpolatedStringElement, Stmt, StmtFunctionDef, StmtMatch,
    StringLiteral,
    helpers::is_compound_statement,
    visitor::{Visitor, walk_expr},
};
use ruff_python_trivia::find_trailing_pragma_offset;
use ruff_source_file::UniversalNewlines;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        comments::trailing_comment,
        docstring::{body_docstring, docstring_slots},
        inline::display_width,
        layout::requires_expand,
        slots::{item_holding, slot_holding},
        walk::walk_stmt,
    },
    rules::{
        Rule, RuleId,
        align_match_case::AlignMatchCase,
        reflow_calls::ReflowCalls,
        reflow_collections::ReflowCollections,
        reflow_imports::ReflowImports,
        reflow_signatures::ReflowSignatures,
        stack_adjacent_strings::{StackAdjacentStrings, concatenated_run},
        wrap_docstrings::WrapDocstrings,
    },
    source::Source,
    suppression::is_directive_comment,
};

mod split;

#[derive(Debug)]
pub(crate) struct LineOverflow {
    code_line_length: usize,
    import_line_length: usize,
    suggest_string_splits: bool,
}

impl LineOverflow {
    pub(crate) const MESSAGE: &'static str =
        "shorten a line that runs past its length budget, or take the string split the fix offers";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            import_line_length: config.import_width(),
            suggest_string_splits: config.rules.line_overflow.suggest_string_splits,
        }
    }
}

impl Rule for LineOverflow {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut spans = Spans {
            docstrings: docstring_slots(&source.ast().body),
            holds: Vec::new(),
            imports: Vec::new(),
            reach: Vec::new(),
            reshapeable: Vec::new(),
            source,
            strings: Vec::new(),
        };
        spans.note_docstring(&source.ast().body);
        spans.note_layouts();
        spans.visit_body(&source.ast().body);
        spans.index();
        let floor = self.code_line_length.min(self.import_line_length);
        source
            .text()
            .universal_newlines()
            .filter_map(|line| {
                let range = line.range();
                let width = display_width(line.as_str());
                if width <= floor {
                    return None;
                }
                let cap = if spans.in_import(range) {
                    self.import_line_length
                } else {
                    self.code_line_length
                };
                if width <= cap
                    || width_before_pragma(source, range).is_some_and(|code| code <= cap)
                    || (spans.reshapes(range) && source.tail_width(range) > cap)
                {
                    return None;
                }
                let report = format!("Line is {width} columns, over the {cap}-column budget");
                let lit = spans.straddling(range, cap);
                Some(
                    match lit.and_then(|lit| split::concatenation(source, lit, cap)) {
                        Some(edit) if self.suggest_string_splits => Diagnostic::suggestion(
                            self.id(),
                            range,
                            format!("{report}, with a legal reshape at the string literal"),
                            edit,
                        ),
                        Some(_) => Diagnostic::lint(self.id(), range, report),
                        None if lit.is_some_and(|lit| split::has_interior_break(source, lit)) => {
                            Diagnostic::lint(self.id(), range, report)
                        }
                        None => Diagnostic::lint(
                            self.id(),
                            range,
                            format!("{report}, {}", spans.kept_because(range, cap)),
                        ),
                    },
                )
            })
            .collect()
    }
}

/// Gathers the import-statement ranges that shift a line to the import
/// budget, the still-collapsible construct ranges a layout rule could
/// shorten, the constructs a suppression holds, the one-line string
/// literals a suggested reshape can split, and the docstring slots a
/// concatenated run is held in.
struct Spans<'a> {
    docstrings: Vec<TextRange>,
    /// Each construct a suppression holds its splitting rule over, with
    /// that rule.
    holds: Vec<(TextRange, RuleId)>,
    imports: Vec<TextRange>,
    /// The furthest end any `reshapeable` range up to each index
    /// covers, so the intersection test is one binary search over the
    /// ascending starts and one read.
    reach: Vec<TextSize>,
    reshapeable: Vec<TextRange>,
    source: &'a Source,
    strings: Vec<&'a StringLiteral>,
}

impl<'a> Spans<'a> {
    /// True for an implicitly concatenated run `stack-adjacent-strings`
    /// still breaks, which leaves out a run filling a docstring slot.
    fn breakable_run(&self, expr: &Expr) -> bool {
        concatenated_run(expr).is_some() && !self.docstrings.contains(&expr.range())
    }

    /// True where a suppression holds `rule` over `range`, which leaves
    /// the construct there as written.
    fn held(&self, range: TextRange, rule: RuleId) -> bool {
        self.source.suppression_map().suppresses(range, rule)
    }

    /// True when `line` meets an import statement, which answers to the
    /// import budget. Import statements never nest, so the last range
    /// opening at or before `line`'s end is the only candidate.
    fn in_import(&self, line: TextRange) -> bool {
        item_holding(&self.imports, line.end()).is_some_and(|import| import.end() >= line.start())
    }

    /// Orders both collected range lists by start and fills [`Self::reach`]
    /// for the per-line binary searches below.
    fn index(&mut self) {
        self.imports.sort_unstable_by_key(Ranged::start);
        self.reshapeable.sort_unstable_by_key(Ranged::start);
        self.reach = self
            .reshapeable
            .iter()
            .scan(TextSize::new(0), |far, r| {
                *far = (*far).max(r.end());
                Some(*far)
            })
            .collect();
    }

    /// The ending a report on `line` closes with where no string split
    /// applies: its trailing comment where its code fits `cap`, the rule
    /// a suppression holds over a construct on it, or no legal reshape.
    fn kept_because(&self, line: TextRange, cap: usize) -> String {
        if self.source.tail_width(line) <= cap {
            return "with only its trailing comment past it".to_owned();
        }
        self.holds
            .iter()
            .find(|(held, _)| held.start() <= line.end() && held.end() >= line.start())
            .map_or_else(
                || "with no legal reshape".to_owned(),
                |(_, rule)| format!("with `{rule}` held by a skip"),
            )
    }

    /// Records a leading docstring's whole range, the prose
    /// `wrap-docstrings` reflows to the budget, where no suppression
    /// holds that rule over it.
    fn note_docstring(&mut self, body: &[Stmt]) {
        if let Some(lit) = body_docstring(body) {
            if self.held(lit.range(), WrapDocstrings::SLUG) {
                self.holds.push((lit.range(), WrapDocstrings::SLUG));
            } else {
                self.reshapeable.push(lit.range());
            }
        }
    }

    /// Records a construct on one source line that a suppression holds
    /// `rule`, the layout rule splitting it, over, along with that rule.
    fn note_held(&mut self, range: TextRange, rule: RuleId) {
        if !self.source.contains_line_break(range) {
            self.holds.push((range, rule));
        }
    }

    /// Records an import statement's `range` as answering to the import
    /// budget, and as reshapeable when two or more `names` form the
    /// comma join `reflow-imports` splits.
    fn note_import(&mut self, range: TextRange, names: usize) {
        self.imports.push(range);
        if names >= 2 {
            self.note_unheld(range, ReflowImports::SLUG);
        }
    }

    /// Records `range` as reshapeable when it sits on one source line,
    /// the form a layout rule can still explode.
    fn note_inline(&mut self, range: TextRange) {
        if !self.source.contains_line_break(range) {
            self.reshapeable.push(range);
        }
    }

    /// Records each argument list `reflow-calls` can explode and each
    /// collection literal `reflow-collections` can expand, wherever it sits
    /// on one source line.
    fn note_layouts(&mut self) {
        for range in self
            .source
            .explodable_arguments()
            .iter()
            .chain(self.source.expandable_literals())
        {
            self.note_inline(*range);
        }
    }

    /// Records a single-statement match arm on one source line, the
    /// form `align-match-case` splits onto the next line.
    fn note_match(&mut self, m: &StmtMatch) {
        for case in &m.cases {
            if let [body] = case.body.as_slice()
                && !is_compound_statement(body)
            {
                self.note_unheld(case.range(), AlignMatchCase::SLUG);
            }
        }
    }

    /// Records a signature carrying parameters, the form
    /// `reflow-signatures` explodes one parameter per line.
    fn note_signature(&mut self, fd: &StmtFunctionDef) {
        if !fd.parameters.is_empty() {
            self.note_unheld(fd.parameters.range(), ReflowSignatures::SLUG);
        }
    }

    /// Records a string literal written as one part on one source line,
    /// the form the adjacent-literal suggestion splits.
    fn note_string(&mut self, expr: &'a ExprStringLiteral) {
        if let [lit] = expr.value.as_slice()
            && !self.source.contains_line_break(lit)
        {
            self.strings.push(lit);
        }
    }

    /// Records `range` through [`Self::note_inline`] unless a suppression
    /// holds `rule`, the layout rule that splits it, over that range, and
    /// through [`Self::note_held`] where one does.
    fn note_unheld(&mut self, range: TextRange, rule: RuleId) {
        if self.held(range, rule) {
            self.note_held(range, rule);
        } else {
            self.note_inline(range);
        }
    }

    /// True when a still-collapsible construct meets `line`, which
    /// leaves the line to the layout rule that shortens it.
    fn reshapes(&self, line: TextRange) -> bool {
        slot_holding(&self.reshapeable, line.end()).is_some_and(|i| self.reach[i] >= line.start())
    }

    /// The single-part string literal on `line` whose span crosses the
    /// `cap` column.
    fn straddling(&self, line: TextRange, cap: usize) -> Option<&StringLiteral> {
        self.strings.iter().copied().find(|lit| {
            line.contains_range(lit.range())
                && self.source.width_between(line.start(), lit.start()) <= cap
                && self.source.width_between(line.start(), lit.end()) > cap
        })
    }
}

impl<'a> Visitor<'a> for Spans<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            _ if self.breakable_run(expr) => {
                self.note_unheld(expr.range(), StackAdjacentStrings::SLUG);
            }
            Expr::Call(call)
                if !call.arguments.is_empty()
                    && self.held(call.arguments.range(), ReflowCalls::SLUG) =>
            {
                self.note_held(call.arguments.range(), ReflowCalls::SLUG);
            }
            Expr::StringLiteral(s) => self.note_string(s),
            _ if requires_expand(expr) && self.held(expr.range(), ReflowCollections::SLUG) => {
                self.note_held(expr.range(), ReflowCollections::SLUG);
            }
            _ => {}
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(cd) => self.note_docstring(&cd.body),
            Stmt::FunctionDef(fd) => {
                self.note_signature(fd);
                self.note_docstring(&fd.body);
            }
            Stmt::Import(i) => self.note_import(i.range(), i.names.len()),
            Stmt::ImportFrom(i) => self.note_import(i.range(), i.names.len()),
            Stmt::Match(m) => self.note_match(m),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the display width of `line` short of a trailing tool pragma
/// or `prose` directive, or `None` where neither trails it.
fn width_before_pragma(source: &Source, line: TextRange) -> Option<usize> {
    let comment = trailing_comment(source, line.start())?;
    let text = source.slice(comment);
    let offset =
        find_trailing_pragma_offset(text).or_else(|| is_directive_comment(text).then_some(0))?;
    let end = comment.start() + TextSize::try_from(offset).ok()?;
    Some(display_width(
        source.slice(TextRange::new(line.start(), end)).trim_end(),
    ))
}
