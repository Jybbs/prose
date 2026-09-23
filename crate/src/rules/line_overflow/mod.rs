//! Flags each physical line still over its cap, `import_line_length`
//! inside an import statement and `code_line_length` elsewhere, once no
//! layout rule can shorten it. A row where a layout rule splits a
//! construct is left to that rule. It is still flagged where that rule is
//! off or a skip holds it there, or where its code fits and only a
//! trailing comment runs past the cap. A line whose code fits ahead of a
//! trailing pragma or `prose` directive is never flagged. A line whose
//! overflow sits inside one string literal holding interior whitespace
//! carries the [`split`] form as a display-only suggestion, gated by
//! `suggest_string_splits`. Lint-only.

use std::{iter, slice};

use itertools::Itertools;
use ruff_python_ast::{
    Alias, Arguments, Expr, ExprStringLiteral, InterpolatedStringElement, Stmt, StmtFunctionDef,
    StmtMatch, StringLike, StringLiteral,
    helpers::is_compound_statement,
    visitor::{Visitor, walk_expr},
};
use ruff_python_trivia::is_pragma_comment;
use ruff_source_file::{LineRanges, UniversalNewlineIterator, UniversalNewlines};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{
        call_keywords::module_call_params,
        comments::{is_keep_marker, trailing_comment},
        docstring::docstring_slots,
        inline::{display_width, spliced_rows},
        padding,
        slots::{item_holding, slot_holding},
        walk::walk_stmt,
    },
    rules::{
        KNOWN_IDS, Rule, RuleId,
        align_match_case::AlignMatchCase,
        expand_docstrings::ExpandDocstrings,
        frame_docstrings::FrameDocstrings,
        reflow_calls::ReflowCalls,
        reflow_collections::ReflowCollections,
        reflow_imports::ReflowImports,
        reflow_signatures::{self, ReflowSignatures},
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
    /// Each rule the configuration leaves off, `reflow-collections`
    /// counting as off wherever it expands no literal.
    off: Vec<RuleId>,
    /// The terms `reflow-signatures` decides a signature's shape under.
    signatures: reflow_signatures::Terms,
    /// True where `reflow-imports` splits a comma-joined `import a, b`.
    splits_bare_imports: bool,
    stranding: padding::Stranding,
    suggest_string_splits: bool,
    /// The rule whose edits name the docstring rows it rewrites.
    wrap_docstrings: WrapDocstrings,
}

impl LineOverflow {
    pub(crate) const MESSAGE: &'static str =
        "shorten a line that runs past its length budget, or take the string split the fix offers";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            import_line_length: config.import_width(),
            off: KNOWN_IDS
                .iter()
                .copied()
                .filter(|&id| {
                    if id == ReflowCollections::SLUG {
                        !config.expands_literals()
                    } else {
                        !config.rules.enabled(id)
                    }
                })
                .collect(),
            signatures: reflow_signatures::Terms::from_config(config),
            splits_bare_imports: config.rules.reflow_imports.split_multi_module,
            stranding: config.stranded_padding(),
            suggest_string_splits: config.rules.line_overflow.suggest_string_splits,
            wrap_docstrings: WrapDocstrings::from_config(config),
        }
    }
}

impl Rule for LineOverflow {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let targets = module_call_params(source);
        let padding = source.stranded_padding(self.stranding);
        let mut spans = Spans {
            blocked: Vec::new(),
            docstrings: docstring_slots(&source.ast().body),
            exploding: self
                .signatures
                .over(source, &targets, &padding)
                .exploding_parameters(&source.ast().body),
            imports: Vec::new(),
            off: &self.off,
            reach: Vec::new(),
            reshapeable: Vec::new(),
            source,
            splits_bare_imports: self.splits_bare_imports,
            strings: Vec::new(),
        };
        spans.note_docstrings(&self.wrap_docstrings);
        spans.visit_body(&source.ast().body);
        spans.index();
        let floor = self.code_line_length.min(self.import_line_length);
        source
            .text()
            .universal_newlines()
            .filter_map(|line| {
                let range = line.range();
                let width = display_width(line.as_str().trim_end());
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
                {
                    return None;
                }
                let kept = spans.kept(range, cap)?;
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
                            format!("{report}, {}", kept.ending()),
                        ),
                    },
                )
            })
            .collect()
    }
}

/// The reason a line over its cap stays as written.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kept {
    /// Only a trailing comment runs past the cap.
    Comment,
    /// A suppression holds this layout rule over a construct on the line.
    Held(RuleId),
    /// No layout rule can shorten the line.
    NoReshape,
    /// The configuration turns off this layout rule, which splits a
    /// construct on the line.
    Off(RuleId),
}

impl Kept {
    /// Returns the clause a report on the line ends with.
    fn ending(self) -> String {
        match self {
            Self::Comment => "with only its trailing comment past it".to_owned(),
            Self::Held(rule) => format!("with `{rule}` held by a skip"),
            Self::NoReshape => "with no legal reshape".to_owned(),
            Self::Off(rule) => format!("with `{rule}` off"),
        }
    }
}

/// Gathers the import-statement ranges that shift a line to the import
/// budget, the places a layout rule splits a row, the constructs no
/// layout rule reaches, the one-line string literals a suggested reshape
/// can split, and the docstring slots.
struct Spans<'a> {
    /// Each place a layout rule would split a row where the rule is off or
    /// held, beside the reason a line holding it stays as written.
    blocked: Vec<(TextRange, Kept)>,
    /// Each docstring slot in source order, whether it holds a docstring or
    /// a concatenated run.
    docstrings: Vec<TextRange>,
    /// The start of each parameter list `reflow-signatures` lays out one
    /// per row, ascending.
    exploding: Vec<TextSize>,
    imports: Vec<TextRange>,
    off: &'a [RuleId],
    /// The furthest end any `reshapeable` range up to each index
    /// covers, so the intersection test is one binary search over the
    /// ascending starts and one read.
    reach: Vec<TextSize>,
    /// Each place a layout rule splits a row, and each docstring row a
    /// docstring rule rewrites.
    reshapeable: Vec<TextRange>,
    source: &'a Source,
    splits_bare_imports: bool,
    strings: Vec<&'a StringLiteral>,
}

impl<'a> Spans<'a> {
    /// Returns the implicitly concatenated run `expr` holds where
    /// `stack-adjacent-strings` still breaks it, which leaves out a run
    /// filling a docstring slot.
    fn breakable_run<'e>(&self, expr: &'e Expr) -> Option<StringLike<'e>> {
        concatenated_run(expr).filter(|_| !self.docstrings.contains(&expr.range()))
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

    /// Returns why `line`, over `cap`, stays as written, or `None` where a
    /// layout rule splits it. A line whose code fits `cap` is kept by its
    /// trailing comment.
    fn kept(&self, line: TextRange, cap: usize) -> Option<Kept> {
        if self.source.tail_width(line) <= cap {
            return Some(Kept::Comment);
        }
        if self.reshapes(line) {
            return None;
        }
        Some(
            self.blocked
                .iter()
                .find(|(range, _)| line.intersect(*range).is_some())
                .map_or(Kept::NoReshape, |&(_, kept)| kept),
        )
    }

    /// Records each of `splits`, the places `rule` shortens a row of the
    /// construct over `range`, through [`Self::record`], counting `rule` off
    /// wherever the configuration turns it off.
    fn note(
        &mut self,
        range: TextRange,
        rule: RuleId,
        splits: impl IntoIterator<Item = TextRange>,
    ) {
        let off = self.off.contains(&rule);
        self.record(range, rule, off, splits);
    }

    /// Records an argument list `reflow-calls` lays out one argument per
    /// row.
    fn note_arguments(&mut self, arguments: &Arguments) {
        let items = arguments.iter_source_order().map(|arg| arg.range());
        let splits = row_gaps(self.source, bracketed(arguments.range(), items));
        self.note(arguments.range(), ReflowCalls::SLUG, splits);
    }

    /// Records each docstring row the docstring rules rewrite. A docstring
    /// `wrap-docstrings` reads only after `frame-docstrings` or
    /// `expand-docstrings` reshapes it leaves every row to that rule. Any
    /// other row whose text survives a `wrap-docstrings` edit is left out.
    fn note_docstrings(&mut self, wrap: &WrapDocstrings) {
        let source = self.source;
        let wrapped = wrap.apply(source).into_iter().flatten().collect_vec();
        let reshaped = [
            (FrameDocstrings::SLUG, FrameDocstrings.apply(source)),
            (ExpandDocstrings::SLUG, ExpandDocstrings.apply(source)),
        ];
        for (rule, groups) in reshaped {
            for edits in groups {
                if let Some(first) = edits.first()
                    && let Some(&slot) = item_holding(&self.docstrings, first.start())
                    && !wrapped.iter().any(|edit| slot.contains_range(edit.range()))
                {
                    self.note(slot, rule, [slot]);
                }
            }
        }
        for edit in wrapped {
            let rewrite = spliced_rows(source, edit.range(), slice::from_ref(&edit));
            let kept: FxHashSet<&str> = rewrite
                .universal_newlines()
                .map(|row| row.as_str())
                .collect();
            let rows = source.text().lines_range(edit.range());
            let splits = UniversalNewlineIterator::with_offset(source.slice(rows), rows.start())
                .filter(|row| !kept.contains(row.as_str()))
                .map(|row| row.range());
            self.note(edit.range(), WrapDocstrings::SLUG, splits);
        }
    }

    /// Records an import statement's `range` as answering to the import
    /// budget. Where two or more `names` form the comma join
    /// `reflow-imports` splits, each gap between names sharing a row is a
    /// place that rule splits. A `bare` import counts that rule off where
    /// its `split-multi-module` facet is unset.
    fn note_import(&mut self, range: TextRange, names: &[Alias], bare: bool) {
        self.imports.push(range);
        if names.len() >= 2 {
            let off =
                self.off.contains(&ReflowImports::SLUG) || (bare && !self.splits_bare_imports);
            let splits = row_gaps(self.source, names.iter().map(Ranged::range));
            self.record(range, ReflowImports::SLUG, off, splits);
        }
    }

    /// Records a collection literal `reflow-collections` lays out one entry
    /// per row, breaking an over-wide dict entry at its `:`.
    fn note_literal(&mut self, expr: &Expr) {
        let elts: &[Expr] = match expr {
            Expr::List(list) => &list.elts,
            Expr::Set(set) => &set.elts,
            Expr::Tuple(tuple) => &tuple.elts,
            _ => &[],
        };
        let entries = expr.as_dict_expr().into_iter().flat_map(|dict| {
            dict.iter().flat_map(|item| {
                item.key
                    .iter()
                    .map(Ranged::range)
                    .chain([item.value.range()])
            })
        });
        let items = elts.iter().map(Ranged::range).chain(entries);
        let splits = row_gaps(self.source, bracketed(expr.range(), items));
        self.note(expr.range(), ReflowCollections::SLUG, splits);
    }

    /// Records each single-statement match arm, whose body
    /// `align-match-case` moves onto the row below its header.
    fn note_match(&mut self, m: &StmtMatch) {
        for case in &m.cases {
            if let [body] = case.body.as_slice()
                && !is_compound_statement(body)
            {
                let header = case
                    .guard
                    .as_deref()
                    .map_or(case.pattern.range(), Ranged::range);
                let splits = row_gaps(self.source, [header, body.range()]);
                self.note(case.range(), AlignMatchCase::SLUG, splits);
            }
        }
    }

    /// Records a signature carrying parameters that `reflow-signatures`
    /// lays out one per row.
    fn note_signature(&mut self, fd: &StmtFunctionDef) {
        let params = &fd.parameters;
        if params.is_empty() || self.exploding.binary_search(&params.start()).is_err() {
            return;
        }
        let items = params.iter_source_order().map(|param| param.range());
        let splits = row_gaps(self.source, bracketed(params.range(), items));
        self.note(params.range(), ReflowSignatures::SLUG, splits);
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

    /// Records each of `splits`, the places `rule` shortens a row of the
    /// construct over `range`. Each is blocked where `off` holds or a
    /// suppression holds `rule` over the construct, and otherwise left to
    /// `rule`.
    fn record(
        &mut self,
        range: TextRange,
        rule: RuleId,
        off: bool,
        splits: impl IntoIterator<Item = TextRange>,
    ) {
        let blocked = if off {
            Some(Kept::Off(rule))
        } else if self.source.suppression_map().suppresses(range, rule) {
            Some(Kept::Held(rule))
        } else {
            None
        };
        match blocked {
            Some(kept) => self
                .blocked
                .extend(splits.into_iter().map(|split| (split, kept))),
            None => self.reshapeable.extend(splits),
        }
    }

    /// True when a place a layout rule splits meets `line`, which leaves
    /// the line to that rule.
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
        if let Some(run) = self.breakable_run(expr) {
            let splits = row_gaps(self.source, run.parts().map(|part| part.range()));
            self.note(expr.range(), StackAdjacentStrings::SLUG, splits);
        } else {
            match expr {
                Expr::Call(call) if self.source.is_explodable(&call.arguments) => {
                    self.note_arguments(&call.arguments);
                }
                Expr::StringLiteral(s) => self.note_string(s),
                _ if self.source.is_expandable(expr) => self.note_literal(expr),
                _ => {}
            }
        }
        walk_expr(self, expr);
    }

    /// Leaves a replacement field unwalked.
    fn visit_interpolated_string_element(&mut self, _: &'a InterpolatedStringElement) {}

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(fd) => self.note_signature(fd),
            Stmt::Import(i) => self.note_import(i.range(), &i.names, true),
            Stmt::ImportFrom(i) => self.note_import(i.range(), &i.names, false),
            Stmt::Match(m) => self.note_match(m),
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

/// Returns the pieces of the bracketed construct over `range`, its opening
/// bracket, then `items`, then its closing bracket.
fn bracketed(
    range: TextRange,
    items: impl IntoIterator<Item = TextRange>,
) -> impl Iterator<Item = TextRange> {
    let bracket = TextSize::from(1);
    iter::once(TextRange::at(range.start(), bracket))
        .chain(items)
        .chain(iter::once(TextRange::at(range.end() - bracket, bracket)))
}

/// Returns the gap between each pair of neighboring `pieces` that share a
/// row, each a place the layout rule over them splits that row.
fn row_gaps(
    source: &Source,
    pieces: impl IntoIterator<Item = TextRange>,
) -> impl Iterator<Item = TextRange> {
    pieces
        .into_iter()
        .tuple_windows()
        .map(|(before, after)| TextRange::new(before.end(), after.start()))
        .filter(|gap| !source.contains_line_break(*gap))
}

/// Returns the display width of `line` short of the run of tool pragmas,
/// `prose` directives, and keep markers its trailing comment ends on, or
/// `None` where that comment ends on an ordinary note or no comment trails
/// the line.
fn width_before_pragma(source: &Source, line: TextRange) -> Option<usize> {
    let comment = trailing_comment(source, line.start())?;
    let text = source.slice(comment);
    let mut end = text.len();
    let offset = text
        .rmatch_indices('#')
        .map_while(|(start, _)| {
            let chunk = text[start..end].trim_end();
            end = start;
            (is_pragma_comment(chunk) || is_directive_comment(chunk) || is_keep_marker(chunk))
                .then_some(start)
        })
        .last()?;
    let end = comment.start() + TextSize::try_from(offset).ok()?;
    Some(display_width(
        source.slice(TextRange::new(line.start(), end)).trim_end(),
    ))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    fn messages(src: &str, config: &str) -> Vec<String> {
        let (config, _) = Config::from_prose_toml_str(config).expect("config parses");
        LineOverflow::from_config(&config)
            .lint(&parse(src))
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[rstest]
    #[case::padding_with_no_comment(&format!("x = 1{}\n", " ".repeat(90)), "", &[])]
    #[case::padding_after_a_short_note(
        &format!("value = alpha + beta  # short note{}\n", " ".repeat(70)),
        "",
        &[],
    )]
    #[case::a_held_row_already_standing_alone(
        "result = some_function_name(\n    an_extremely_long_single_argument_expression_name_that_runs_past_the_limit,\n    second,\n)  # prose: skip[reflow-calls]\n",
        "code-line-length = 70",
        &["Line is 79 columns, over the 70-column budget, with no legal reshape"],
    )]
    #[case::a_keep_marker_ending_a_header(
        "dispatch_table_for_the_request_handlers_in_order_of_precedence_here_too = {  # prose: keep\n    \"fetch\": fetch_payload,\n    \"parse\": parse_records,\n}\n",
        "",
        &[],
    )]
    #[case::a_bare_import_left_joined(
        "import collections_long_module_name_one, collections_long_module_name_two\n",
        "import-line-length = 60\n\n[rules]\nreflow-imports = { split-multi-module = false }\n",
        &["Line is 73 columns, over the 60-column budget, with `reflow-imports` off"],
    )]
    #[case::a_row_holding_two_arguments(
        "result = combine(\n    first_argument_value_long_name, second_argument_value_long_name, third_argu,\n)\n",
        "code-line-length = 80",
        &[],
    )]
    #[case::a_row_holding_one_argument(
        "result = combine(\n    first_argument_value_long_name_second_argument_value_long_name_third_argument,\n)\n",
        "code-line-length = 70",
        &["Line is 82 columns, over the 70-column budget, with no legal reshape"],
    )]
    #[case::a_dict_entry_split_at_its_colon(
        "table = {\n    \"first_key_long_enough_to_matter\": first_value_long_enough_to_matter_here,\n}\n",
        "code-line-length = 80",
        &[],
    )]
    #[case::a_sole_generator_argument(
        "total = compute(value_long_enough_to_matter for value_long_enough_to_matter in source_values)\n",
        "",
        &[],
    )]
    #[case::a_stub_whose_body_carries_the_row_past(
        "def configure_request(first_parameter_value, second_parameter_value, third) -> Mapping: ...\n",
        "",
        &["Line is 91 columns, over the 88-column budget, with no legal reshape"],
    )]
    #[case::a_layout_rule_turned_off(
        "result = combine(first_argument_value_long_name, second_argument_value_long_name, third_value)\n",
        "[rules]\nreflow-calls = false\n",
        &["Line is 94 columns, over the 88-column budget, with `reflow-calls` off"],
    )]
    #[case::a_url_row_between_rewrapped_paragraphs(
        "def fetch():\n    \"\"\"\n    Fetch the resource this module names from its canonical location on the network, retrying.\n\n    https://example.com/a/deeply/nested/path/that/runs/well/past/the/docstring/budget/index.html\n\n    Return the parsed payload once the server answers, raising on any status past the budget.\n    \"\"\"\n",
        "",
        &["Line is 96 columns, over the 88-column budget, with no legal reshape"],
    )]
    #[case::an_unframed_docstring(
        "def fetch():\n    \"\"\"Fetch the resource this module names from its canonical location on the network.\n\n    https://example.com/a/deeply/nested/path/that/runs/well/past/the/docstring/budget/index.html\n    \"\"\"\n",
        "",
        &[],
    )]
    #[case::a_one_line_docstring_with_expansion_off(
        "def locate():\n    \"\"\"https://example.com/a/deeply/nested/path/that/runs/well/past/the/docstring/budget/index\"\"\"\n",
        "[rules]\nexpand-docstrings = false\n",
        &["Line is 97 columns, over the 88-column budget, with `expand-docstrings` off"],
    )]
    #[case::docstring_wrapping_turned_off(
        "def locate():\n    \"\"\"\n    Locate the configuration this module carries, in a sentence long enough to overflow its row.\n    \"\"\"\n",
        "[rules]\nwrap-docstrings = false\n",
        &["Line is 96 columns, over the 88-column budget, with `wrap-docstrings` off"],
    )]
    #[case::expansion_turned_off(
        "values = [first_argument_value_long_name, second_argument_value_long_name, third_item_value]\n",
        "[rules]\nreflow-collections = { explode = false }\n",
        &["Line is 92 columns, over the 88-column budget, with `reflow-collections` off"],
    )]
    fn lint_reads_each_row_a_layout_rule_splits(
        #[case] src: &str,
        #[case] config: &str,
        #[case] expected: &[&str],
    ) {
        assert_eq!(messages(src, config), expected);
    }
}
