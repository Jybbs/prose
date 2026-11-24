//! Reflows a redundant grouping parenthesis pair against the line
//! budget, the pair whose removal leaves both the parse and the
//! grouping a reader sees unchanged, so a precedence pair, a generator,
//! a walrus binding, a one-element tuple, and a pair holding one
//! boolean operator inside a chain at the other all survive. A wrapped
//! pair folds onto one line when the bare form fits, one holding an
//! over-budget operator chain or conditional expression breaks across
//! rows one operand per row, and one whose interior breaks sit inside a
//! bracket of its own sheds in place. A pair holding the whole value
//! of a `return` or an assignment that is one bracketed construct
//! landing on one row or across rows comes off, that construct
//! opening on the statement's row, unless that row then runs past the
//! budget or rows no later rule closes cannot hang from the statement's
//! indent. An over-budget boolean chain or conditional forming a test,
//! a `return` value, an assignment value, or the last element of a bare
//! tuple standing as one gains a pair the break adds. Each row is
//! measured as the later rules shortening it leave it.
//! `plan` finds the pairs, `chain` divides an interior into operands,
//! `measure` answers whether each direction fits, and `render` builds
//! the broken text.

use std::{cell::RefCell, mem};

use ruff_diagnostics::{Edit, SourceMap};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    pipeline::prepared_groups,
    primitives::{
        call_keywords::module_call_params,
        edit::{insert_edit, singleton_groups},
        fracture::outermost,
        one_row, padding,
        reseat::push_reseat_edits,
        reserve,
    },
    rules::{
        Rule, RuleId, prefer_fstring::PreferFstring, shed_super_args::ShedSuperArgs,
        simplify_comprehensions::SimplifyComprehensions,
    },
    source::Source,
};

mod chain;
mod flush;
mod measure;
mod plan;
mod render;

use plan::{Candidate, candidates, outermost_calls};

#[derive(Debug)]
pub(crate) struct ReflowParentheses {
    code_line_length: usize,
    fstrings: PreferFstring,
    one_row: one_row::Settings<'static>,
    reflows_calls: bool,
    reservations: reserve::Reservations,
    sheds_super_args: bool,
    simplifies_comprehensions: bool,
    stranding: padding::Stranding,
}

impl ReflowParentheses {
    pub(crate) const MESSAGE: &'static str =
        "remove a redundant grouping parenthesis pair, or break one that runs past the line budget";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) const PRESERVES_TREE: bool = true;

    pub(crate) fn from_config(config: &Config) -> Self {
        Self {
            code_line_length: config.code_width(),
            fstrings: config.fstrings(),
            one_row: config.one_row_settings(),
            reflows_calls: config.rules.reflow_calls.enabled,
            reservations: config.equals_reservations(),
            sheds_super_args: config.rules.shed_super_args.enabled,
            simplifies_comprehensions: config.rules.simplify_comprehensions.enabled,
            stranding: config.stranded_padding(),
        }
    }

    /// Runs `measure` over a [`Shedder`] holding no edits yet, built over
    /// `source` with every table these settings read off it, `simplified`
    /// the fix groups `simplify-comprehensions` emits over `source`, and
    /// `landed` that source with each of those groups applied.
    fn measuring<R>(
        &self,
        source: &Source,
        simplified: &[Vec<Edit>],
        landed: Option<Landed<'_>>,
        measure: impl FnOnce(&mut Shedder) -> R,
    ) -> R {
        let calls = if self.reflows_calls {
            outermost_calls(source)
        } else {
            Vec::new()
        };
        let stranded = source.stranded_padding(self.stranding);
        let fstrings = source.fstring_rewrites(self.fstrings);
        let rewrites = self.rewrites(source, simplified, &fstrings);
        let padding = padding::beside(&stranded, &rewrites);
        let reservations = source.columns(&self.reservations);
        let targets = module_call_params(source);
        measure(&mut Shedder {
            breaks: RefCell::default(),
            calls,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            folds: Vec::new(),
            landed,
            one_row: self.one_row.against(&targets).forecasting(&fstrings),
            padding: &padding,
            reservations: &reservations,
            simplified,
            source,
        })
    }

    /// The edits the rules shortening a row after this one land over
    /// `source`, ascending and disjoint, an edit inside another left
    /// out. These are `fstrings`, the f-strings `prefer-fstring`
    /// forecasts, the edits of each fix group `shed-super-args` emits
    /// that no skip directive holds, and the edits of `simplified`,
    /// leaving out each of the latter two that covers a line break.
    fn rewrites(&self, source: &Source, simplified: &[Vec<Edit>], fstrings: &[Edit]) -> Vec<Edit> {
        let shed = if self.sheds_super_args {
            prepared_groups(&ShedSuperArgs, source)
        } else {
            Vec::new()
        };
        outermost(
            shed.iter()
                .chain(simplified)
                .flatten()
                .filter(|edit| !source.contains_line_break(edit.range()))
                .chain(fstrings)
                .cloned()
                .collect(),
        )
    }
}

impl Rule for ReflowParentheses {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let candidates = candidates(source);
        if candidates.is_empty() {
            return Vec::new();
        }
        let simplified = if self.simplifies_comprehensions {
            prepared_groups(&SimplifyComprehensions, source)
        } else {
            Vec::new()
        };
        let shed = |landed: Option<Landed<'_>>| {
            self.measuring(source, &simplified, landed, |shedder| {
                shedder.shed(&candidates);
                singleton_groups(outermost(mem::take(&mut shedder.edits)))
            })
        };
        let Some((text, map)) = (!simplified.is_empty())
            .then(|| source.editing(simplified.iter().flatten().cloned().collect()))
            .flatten()
        else {
            return shed(None);
        };
        let landing = plan::candidates(&text);
        self.measuring(&text, &[], None, |shedder| {
            shed(Some(Landed {
                candidates: &landing,
                map: &map,
                shedder,
            }))
        })
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// Turns a candidate list into edits, walking it in source order so each
/// budget test reads the columns the preceding edits produce. `calls`
/// holds the outermost call ranges `reflow-calls` explodes where their
/// row overflows, empty where that rule is off. `padding` is every edit
/// `strip-stranded-padding` emits over the source beside the
/// [`ReflowParentheses::rewrites`] later rules land, and `reservations`
/// the columns `align-equals` shifts each aligned value to, the two
/// tables every measure settles a row through. `one_row` holds the terms
/// the layout rules join a bracketed construct onto one row under,
/// `simplified` the fix groups `simplify-comprehensions` emits, `landed`
/// the module those groups rewrite, and `breaks` holds, for the
/// candidate under test, whether each later pair on its row breaks.
struct Shedder<'a> {
    breaks: RefCell<FxHashMap<TextRange, bool>>,
    calls: Vec<TextRange>,
    code_line_length: usize,
    edits: Vec<Edit>,
    folds: Vec<TextRange>,
    landed: Option<Landed<'a>>,
    one_row: one_row::Settings<'a>,
    padding: &'a [Edit],
    reservations: &'a reserve::Columns,
    simplified: &'a [Vec<Edit>],
    source: &'a Source,
}

/// The module `simplify-comprehensions` rewrites, measured once: its
/// `candidates`, the `map` from the written offsets to its own, and the
/// `shedder` built over it.
#[derive(Clone, Copy)]
struct Landed<'a> {
    candidates: &'a [Candidate<'a>],
    map: &'a SourceMap,
    shedder: &'a Shedder<'a>,
}

impl Shedder<'_> {
    /// Emits the edits for every candidate, folding a wrapped pair whose
    /// joined line fits the budget. A candidate inside an open fold
    /// drops its parentheses alone, leaving that fold's own edits to
    /// close the break. A pair whose joined line overflows breaks across
    /// rows where it holds an operator chain or a conditional expression.
    /// Otherwise a pair the source wrote sheds in place where an enclosing
    /// bracket holds its breaks, re-seating the rows its parens moved.
    /// Where none does, a pair holding the whole value of a `return` or an
    /// assignment that is one bracketed construct landing on one row or
    /// across rows opens that bracket on the statement's row, and any
    /// other pair holds, whereas a pair the break adds earns nothing.
    fn shed(&mut self, candidates: &[Candidate]) {
        for candidate in candidates {
            let pair = candidate.pair;
            self.breaks.get_mut().clear();
            self.folds.retain(|fold| fold.contains_range(pair));
            let collapsing = !self.folds.is_empty();
            if !collapsing
                && self.overflows(candidate, candidates)
                && self.push_break_edits(candidate, candidates)
            {
                continue;
            }
            if !candidate.sheds {
                continue;
            }
            let mut folding = !collapsing && self.source.contains_line_break(pair);
            let removals = if collapsing {
                candidate.lone_paren_removals()
            } else if folding && !self.fits(candidate, candidates) {
                let Some((open, close)) = self.shed_in_place_spans(candidate) else {
                    self.push_opened_edits(candidate, candidates);
                    continue;
                };
                folding = false;
                candidate.flush.flanking(open, close)
            } else {
                candidate.paren_removals()
            };
            if !collapsing && !folding {
                push_reseat_edits(self.source, &removals, &mut self.edits);
            }
            for removal in removals {
                insert_edit(&mut self.edits, removal);
            }
            if folding {
                self.push_fold_edits(candidate);
                self.folds.push(pair);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        num::NonZeroUsize,
        time::{Duration, Instant},
    };

    use rstest::rstest;
    use ruff_python_ast::PythonVersion;

    use super::*;
    use crate::{
        pipeline::Pipeline,
        testing::{applied_text, parse},
    };

    /// The paired `dict(...)` values the cost test generates.
    const PAIRED_DICT_CALLS: usize = 500;

    /// The time the rule takes over [`PAIRED_DICT_CALLS`] values under the
    /// test profile, 0.24 s measured with one reparse per module, against
    /// 117 s with one reparse per candidate.
    const PAIRED_DICT_CALLS_BOUND: Duration = Duration::from_secs(10);

    /// `src` with every edit the rule emits applied, under the default
    /// configuration with `reflow-calls` set to `reflows_calls`.
    fn applied(src: &str, reflows_calls: bool) -> String {
        configured(src, |config| {
            config.rules.reflow_calls.enabled = reflows_calls
        })
    }

    /// `src` with every edit the rule emits applied, under the default
    /// configuration as `configure` leaves it.
    fn configured(src: &str, configure: impl FnOnce(&mut Config)) -> String {
        let mut config = Config::default();
        configure(&mut config);
        let source = parse(src);
        let edits = ReflowParentheses::from_config(&config)
            .apply(&source)
            .into_iter()
            .flatten()
            .collect();
        applied_text(&source, edits)
    }

    /// `value` as the whole value of an assignment, or of a `return`
    /// where `returns` holds, inside a redundant pair whose interior
    /// takes rows of its own.
    fn paired(value: &str, returns: bool) -> String {
        let indent = if returns { "        " } else { "    " };
        let rows = value
            .lines()
            .map(|row| format!("{indent}{row}"))
            .collect::<Vec<_>>()
            .join("\n");
        if returns {
            format!("def f():\n    return (\n{rows}\n    )\n")
        } else {
            format!("value = (\n{rows}\n)\n")
        }
    }

    /// Each width from 30 through 88 beside the text the default pipeline
    /// formats [`paired`] to there, with `reflow-calls` set to
    /// `reflows_calls`, asserting a second run leaves that text as it is.
    fn settled_at_every_width(
        value: &str,
        returns: bool,
        reflows_calls: bool,
    ) -> Vec<(usize, String)> {
        (30..=88)
            .map(|width| {
                let mut config = Config {
                    code_line_length: NonZeroUsize::new(width),
                    ..Config::default()
                };
                config.rules.reflow_calls.enabled = reflows_calls;
                let pipeline = Pipeline::with_defaults(&config);
                let src = paired(value, returns);
                let once = pipeline.format(parse(&src)).expect("the first run formats");
                let twice = pipeline
                    .format(parse(once.text()))
                    .expect("the second run formats");
                assert_eq!(twice.text(), once.text(), "width {width}, from:\n{src}");
                (width, once.text().to_owned())
            })
            .collect()
    }

    #[rstest]
    #[case::flush_column_call(
        "value = (\n    compute(\n        a,\n        b,\n    )\n)\n",
        true,
        "value = compute(\n    a,\n    b,\n)\n"
    )]
    #[case::call_past_the_indented_row(
        "value = (\n    compute_something(alpha_argument, delta_argument, epsilon_argument, gamma_argument_x)\n)\n",
        true,
        "value = compute_something(alpha_argument, delta_argument, epsilon_argument, gamma_argument_x)\n"
    )]
    #[case::fractured_call_with_reflow_calls_off(
        "value = (\n    compute(a,\n        b)\n)\n",
        false,
        "value = compute(a,\n    b)\n"
    )]
    #[case::fractured_call_reflow_calls_joins(
        "value = (\n    compute(a,\n        b)\n)\n",
        true,
        "value = compute(a,\n    b)\n"
    )]
    #[case::call_on_a_flush_column_call(
        "value = (\n    factory(\n        alpha,\n        beta,\n    )()\n)\n",
        true,
        "value = factory(\n    alpha,\n    beta,\n)()\n"
    )]
    #[case::call_behind_a_subscript_opener_past_the_indented_row(
        "value = (\n    handlers[lookup(alpha_argument, beta_argument, gamma_argument, delta_argument_xxxxx)]()\n)\n",
        true,
        "value = handlers[lookup(alpha_argument, beta_argument, gamma_argument, delta_argument_xxxxx)]()\n"
    )]
    #[case::call_behind_a_lambda_opener_past_the_indented_row(
        "value = (\n    (lambda: compute(alpha_argument, beta_argument, gamma_argument, delta_argumentxxxx))()\n)\n",
        true,
        "value = (lambda: compute(alpha_argument, beta_argument, gamma_argument, delta_argumentxxxx))()\n"
    )]
    #[case::dict_call_whose_literal_opens_inside_the_budget(
        &format!("value_{0} = (\n    dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4, gamma_value=5, zeta_value=6)\n)\n", "k".repeat(76)),
        true,
        &format!("value_{0} = dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4, gamma_value=5, zeta_value=6)\n", "k".repeat(76))
    )]
    #[case::opened_row_at_the_budget(
        &format!("value_{0} = (\n    loop.call_later(timeout_argument, self._handle_timeout_argument, other_argument_xxxx)\n)\n", "k".repeat(63)),
        true,
        &format!("value_{0} = loop.call_later(timeout_argument, self._handle_timeout_argument, other_argument_xxxx)\n", "k".repeat(63))
    )]
    #[case::pair_past_the_budget_behind_a_call_reflow_calls_explodes(
        "table[resolve(alpha_argument, beta_argument, gamma_argument, delta_argument_xxxxxxxxxxxx)] = (\n    loop.call_later(timeout_argument, self._handle_timeout_argument, other_argument_xxxx)\n)\n",
        true,
        "table[resolve(alpha_argument, beta_argument, gamma_argument, delta_argument_xxxxxxxxxxxx)] = loop.call_later(timeout_argument, self._handle_timeout_argument, other_argument_xxxx)\n"
    )]
    #[case::dict_call_whose_brace_lands_at_the_budget(
        &format!("value_{0} = (\n    dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4, gamma_value=5, zeta_value=6)\n)\n", "k".repeat(78)),
        true,
        &format!("value_{0} = dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4, gamma_value=5, zeta_value=6)\n", "k".repeat(78))
    )]
    #[case::fractured_call_under_a_long_target_with_reflow_calls_off(
        &format!("value_{} = (\n    compute(alpha,\n        beta)\n)\n", "k".repeat(60)),
        false,
        &format!("value_{0} = compute(alpha,\n    beta)\n", "k".repeat(60))
    )]
    #[case::fractured_call_rejoining_at_the_budget_under_a_long_target(
        &format!("value_{} = (\n    compute(alpha,\n        beta)\n)\n", "k".repeat(59)),
        true,
        &format!("value_{0} = compute(alpha,\n    beta)\n", "k".repeat(59))
    )]
    fn a_call_landing_across_rows_opens_on_the_assignment_row(
        #[case] src: &str,
        #[case] reflows_calls: bool,
        #[case] expected: &str,
    ) {
        assert_eq!(applied(src, reflows_calls), expected);
    }

    #[rstest]
    #[case::annotated_column_dict(
        "value: dict = (\n    {\n        \"a\": 1,\n        \"b\": 2,\n    }\n)\n",
        "value: dict = {\n    \"a\": 1,\n    \"b\": 2,\n}\n"
    )]
    #[case::augmented_column_list(
        "total += (\n    [\n        first,\n        second,\n    ]\n)\n",
        "total += [\n    first,\n    second,\n]\n"
    )]
    #[case::fractured_dict_past_the_indented_row(
        "value = (\n    {\"alpha_value_name_xxxxxxx\": 1, \"delta_value_name_xxxxxxxx\": 2,\n     \"epsilon_value_name\": 3}\n)\n",
        "value = {\"alpha_value_name_xxxxxxx\": 1, \"delta_value_name_xxxxxxxx\": 2,\n    \"epsilon_value_name\": 3}\n"
    )]
    fn a_collection_landing_across_rows_opens_on_the_assignment_row(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(applied(src, true), expected);
    }

    #[rstest]
    #[case::from_a_row_of_its_own(
        "def f():\n    return (\n        compute([item.name_of_the_record for item in fetch(\n        ) if item.enabled_for_the_request], key_argument)\n    )\n"
    )]
    #[case::from_the_row_of_the_pair(
        "def f():\n    return (compute([item.name_of_the_record for item in fetch(\n    ) if item.enabled_for_the_request], key_argument)\n    )\n"
    )]
    fn a_call_whose_rows_close_moves_them_to_the_statement_indent(#[case] src: &str) {
        assert_eq!(
            applied(src, true),
            "def f():\n    return compute([item.name_of_the_record for item in fetch(\n    ) if item.enabled_for_the_request], key_argument)\n",
        );
    }

    #[rstest]
    #[case::call(
        "def f():\n    return (\n        compute(\n            a,\n            b,\n        )\n    )\n",
        "def f():\n    return compute(\n        a,\n        b,\n    )\n"
    )]
    #[case::column_dict(
        "def f():\n    return (\n        {\n            \"a\": 1,\n            \"b\": 2,\n        }\n    )\n",
        "def f():\n    return {\n        \"a\": 1,\n        \"b\": 2,\n    }\n"
    )]
    #[case::call_flush_against_the_keyword(
        "def f():\n    return(\n        compute(\n            a,\n            b,\n        )\n    )\n",
        "def f():\n    return compute(\n        a,\n        b,\n    )\n"
    )]
    fn a_construct_landing_across_rows_opens_on_the_return_row(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(applied(src, true), expected);
    }

    #[rstest]
    #[case::fractured_call_holding_a_comment(
        "value = (\n    compute(a,  # note\n        b)\n)\n",
        true
    )]
    #[case::fractured_list_comprehension_holding_a_comment(
        "value = (\n    [item.name  # note\n     for item in items]\n)\n",
        true
    )]
    #[case::fractured_dict_rejoining_inside(
        &format!("value_{} = (\n    {{\"alpha\": 1,\n     \"delta\": 2}}\n)\n", "k".repeat(60)),
        true
    )]
    #[case::dict_call_rejoining_inside(
        &format!("value_{} = (\n    dict(\n        alpha=1,\n        delta=2\n    )\n)\n", "k".repeat(70)),
        true
    )]
    #[case::dict_call_whose_written_rows_outrun_the_row(
        &format!("value_{} = (\n    dict(\n        alpha_value_name=1,\n        delta_value_name=2,\n        epsilon_value_xxxxxx=3\n    )\n)\n", "k".repeat(70)),
        true
    )]
    #[case::call_without_arguments(&format!("value = (\n    module.{}()\n)\n", "k".repeat(80)), true)]
    #[case::subscript(&format!("value = (\n    table[{}]\n)\n", "k".repeat(84)), true)]
    #[case::if_test_column_dict(
        "if (\n    {\n        \"a\": 1,\n        \"b\": 2,\n    }\n):\n    pass\n",
        true
    )]
    #[case::fitting_dict_under_a_long_target(
        &format!("value_{} = (\n    {{\"a\": 1}}\n)\n", "k".repeat(80)),
        true
    )]
    #[case::one_row_call_with_reflow_calls_off(
        &format!("value_{} = (\n    compute(alpha, beta)\n)\n", "k".repeat(80)),
        false
    )]
    #[case::call_behind_a_subscript_opener(
        "value = (\n    handlers[lookup(\n        alpha,\n        beta,\n    )]()\n)\n",
        true
    )]
    #[case::call_behind_a_lambda_opener(
        "value = (\n    (lambda: compute(\n        alpha,\n        beta,\n    ))()\n)\n",
        true
    )]
    #[case::call_behind_a_list_opener(
        "value = (\n    [compute(\n        a,\n        b,\n    )]\n)\n",
        true
    )]
    #[case::exploded_chain_past_the_budget_with_reflow_calls_off(
        &format!("value_{0} = (\n    compute(alpha).transform_everything_now(\n        beta,\n        gamma,\n    )\n)\n", "k".repeat(50)),
        false
    )]
    #[case::opened_row_one_past_the_budget(
        &format!("value_{0} = (\n    loop.call_later(timeout_argument, self._handle_timeout_argument, other_argument_xxxx)\n)\n", "k".repeat(64)),
        true
    )]
    #[case::dict_call_whose_brace_lands_one_past_the_budget(
        &format!("value_{0} = (\n    dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4, gamma_value=5, zeta_value=6)\n)\n", "k".repeat(79)),
        true
    )]
    #[case::fractured_dict_rejoining_one_past_the_budget_under_a_long_target(
        &format!("value_{} = (\n    {{\"alpha\": 1,\n     \"delta\": 2}}\n)\n", "k".repeat(56)),
        true
    )]
    #[case::fractured_call_rejoining_under_a_long_target(
        &format!("value_{} = (\n    compute(alpha,\n        beta)\n)\n", "k".repeat(60)),
        true
    )]
    #[case::method_with_empty_brackets_across_rows_on_a_fractured_call(
        "value = (\n    compute(a,\n        b).run(\n    )\n)\n",
        true
    )]
    #[case::fractured_call_with_a_closer_row_and_reflow_calls_off(
        "value = (\n    compute(lookup(\n        alpha, other(beta\n    ), gamma), key)\n)\n",
        false
    )]
    #[case::call_with_empty_brackets_across_rows("value = (\n    compute(\n    )\n)\n", true)]
    #[case::call_on_a_fractured_dict_reflow_collections_skips(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}.get(key)\n)  # prose: skip[reflow-collections]\n",
        true
    )]
    #[case::call_on_a_comprehension_too_wide_to_rejoin(
        "value = (\n    [item.attribute_name_long for item in fetch_every_record_from_storage(\n    ) if item.compute_the_predicate_value(\n    )].count(key_argument)\n)\n",
        true
    )]
    #[case::call_on_a_comprehension_whose_argument_list_stays_closed(
        "value = (\n    [item.attribute_name for item in fetch_every_record(\n    ) if item.enabled].count(key_argument_name)\n)\n",
        true
    )]
    #[case::count_exploded_call_on_a_comprehension_too_wide_to_rejoin(
        "value = (\n    [item.attribute_name_long for item in fetch_every_record_from_storage(\n    ) if item.compute_the_predicate_value(\n    )].run(a=1, b=2, c=3, d=4)\n)\n",
        true
    )]
    #[case::fractured_dict_reflow_collections_skips(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}\n)  # prose: skip[reflow-collections]\n",
        true
    )]
    #[case::fractured_list_comprehension_reflow_collections_skips(
        "value = (\n    [item.name\n     for item in items]\n)  # prose: skip[reflow-collections]\n",
        true
    )]
    #[case::fractured_subscript_reflow_collections_skips(
        "value = (\n    table[alpha,\n          beta]\n)  # prose: skip[reflow-collections]\n",
        true
    )]
    #[case::comprehension_over_a_call_with_empty_brackets_reflow_collections_skips(
        "value = (\n    [x.name for x in fetch(\n    )]\n)  # prose: skip[reflow-collections]\n",
        true
    )]
    #[case::call_with_empty_brackets_across_rows_on_a_fractured_comprehension(
        "value = (\n    [item.name\n     for item in items].pop(\n    )\n)\n",
        true
    )]
    #[case::call_without_arguments_on_a_fractured_dict_reflow_collections_skips(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}.items()\n)  # prose: skip[reflow-collections]\n",
        true
    )]
    #[case::call_without_arguments_on_a_fractured_dict_rejoining_under_a_long_target(
        &format!("value_{} = (\n    {{\"alpha\": 1,\n     \"delta\": 2}}.items()\n)\n", "k".repeat(60)),
        true
    )]
    #[case::call_without_arguments_on_a_subscript_holding_an_expanding_dict(
        "value = (\n    table[{\"alpha_key_name\": alpha_value_xxxxxxxx, \"delta_key_name\": delta_value_xxxxxxxx}].copy()\n)\n",
        true
    )]
    #[case::fractured_call_reflow_calls_skips(
        "value = (\n    compute(alpha,\n        beta)\n)  # prose: skip[reflow-calls]\n",
        true
    )]
    #[case::call_with_empty_brackets_across_rows_on_a_dict_past_the_budget(
        "value = (\n    {\"alpha_key_name\": alpha_value_xxxxxxxx,\n     \"delta_key_name\": delta_value_xxxxxxxxxxxx}.items(\n    )\n)\n",
        true
    )]
    #[case::column_dict_holding_a_call_exploded_onto_rows(
        "value = (\n    {\n        \"alpha\": compute(\n            beta,\n            gamma,\n        )\n    }\n)\n",
        true
    )]
    fn a_pair_around_any_other_value_holds(#[case] src: &str, #[case] reflows_calls: bool) {
        assert_eq!(applied(src, reflows_calls), src);
    }

    #[rstest]
    #[case::fractured_dict_with_reflow_collections_off(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}\n)\n",
        |config: &mut Config| config.rules.reflow_collections.enabled = false,
        "value = {\"alpha\": 1,\n    \"delta\": 2}\n"
    )]
    #[case::dict_call_with_reflow_collections_off(
        &format!("value_{0} = (\n    dict(\n        alpha=1,\n        delta=2\n    )\n)\n", "k".repeat(60)),
        |config: &mut Config| config.rules.reflow_collections.enabled = false,
        &format!("value_{0} = dict(\n    alpha=1,\n    delta=2\n)\n", "k".repeat(60))
    )]
    #[case::spanning_dict_call_past_the_budget_with_explode_off(
        "value = (\n    dict(\n        alpha_value_long_name=1,\n        delta_value_long_name=2,\n        epsilon_value_name_xxxxx=3\n    )\n)\n",
        |config: &mut Config| config.rules.reflow_collections.explode = false,
        "value = dict(\n    alpha_value_long_name=1,\n    delta_value_long_name=2,\n    epsilon_value_name_xxxxx=3\n)\n"
    )]
    #[case::one_row_call_on_an_expanding_dict_with_reflow_calls_off(
        "value = (\n    {\"alpha\": 1, \"beta\": 2, \"gamma\": 3}.get(key_argument, default_value)\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
        },
        "value = {\"alpha\": 1, \"beta\": 2, \"gamma\": 3}.get(key_argument, default_value)\n"
    )]
    #[case::one_row_items_call_on_an_expanding_dict_with_reflow_calls_off(
        "value = (\n    {\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4}.items()\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
        },
        "value = {\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4}.items()\n"
    )]
    #[case::one_row_call_on_a_dict_its_callee_carries_past_the_budget_with_reflow_calls_off(
        "value = (\n    transform_values({\"alpha\": 1, \"beta\": 22})\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform_values({\"alpha\": 1, \"beta\": 22})\n"
    )]
    #[case::one_row_call_on_a_dict_past_the_indented_row_with_reflow_calls_off(
        "value = (\n    transform({\"alpha\": 1, \"beta\": 2222})\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform({\"alpha\": 1, \"beta\": 2222})\n"
    )]
    #[case::call_on_a_dict_call_of_pairs_past_the_entry_cap(
        "value = (\n    transform(dict([(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)]))\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(62),
        "value = transform(dict([(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)]))\n"
    )]
    #[case::one_row_call_on_a_dict_call_past_the_entry_cap_with_reflow_calls_off(
        "value = (\n    transform(dict(alpha=1, beta=2, gamma=3, delta=4))\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(50);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform(dict(alpha=1, beta=2, gamma=3, delta=4))\n"
    )]
    #[case::one_row_call_on_a_dict_call_past_the_indented_row_with_reflow_calls_off(
        "value = (\n    transform(dict(alpha_value=1, beta_value=2))\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform(dict(alpha_value=1, beta_value=2))\n"
    )]
    #[case::dict_call_on_a_dict_past_the_entry_cap(
        "value = (\n    dict({\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4})\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(52),
        "value = dict({\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4})\n"
    )]
    #[case::dict_call_of_pairs_past_the_entry_cap(
        "value = (\n    dict([(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)])\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(52),
        "value = dict([(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)])\n"
    )]
    #[case::one_row_call_on_a_dict_call_inside_the_indented_row_past_the_entry_cap_with_reflow_calls_off(
        "value = (\n    transform(dict(alpha=1, beta=2, gamma=3, delta=4))\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(62);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform(dict(alpha=1, beta=2, gamma=3, delta=4))\n"
    )]
    #[case::call_on_a_dict_comprehension_call_whose_rewrite_opens_inside_the_budget(
        "value = (\n    dict((key, val.attr) for key, val in some_items_name).get(argument_x, default_value)\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(62),
        "value = dict((key, val.attr) for key, val in some_items_name).get(argument_x, default_value)\n"
    )]
    #[case::spanning_call_on_a_dict_comprehension_call_whose_rewrite_opens_inside_the_budget_with_reflow_calls_off(
        "value = (\n    transform(dict((key, val.attr) for key, val in some_items), second_argument,\n        third)\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(80);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform(dict((key, val.attr) for key, val in some_items), second_argument,\n    third)\n"
    )]
    #[case::call_on_a_list_call_simplify_comprehensions_collapses(
        "value = (\n    list(item for item in some_items_name).index(argument_x, default_value)\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(32),
        "value = list(item for item in some_items_name).index(argument_x, default_value)\n"
    )]
    #[case::dict_call_on_a_nested_dict_call(
        "value = (\n    dict(alpha_value=dict(beta_value=1, gamma_value=2))\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(54),
        "value = dict(alpha_value=dict(beta_value=1, gamma_value=2))\n"
    )]
    #[case::dict_call_on_a_nested_dict_past_the_entry_cap(
        "value = (\n    dict(alpha_value={\"x\": 1, \"y\": 2, \"z\": 3, \"w\": 4})\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(53),
        "value = dict(alpha_value={\"x\": 1, \"y\": 2, \"z\": 3, \"w\": 4})\n"
    )]
    #[case::spanning_call_on_a_dict_call_measured_at_its_literal(
        "value = (\n    transform(dict(alpha=1, beta=2), second_argument_name,\n        third)\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(65),
        "value = transform(dict(alpha=1, beta=2), second_argument_name,\n    third)\n"
    )]
    #[case::one_row_call_on_a_one_entry_dict_call_past_the_indented_row_with_reflow_calls_off(
        "value = (\n    transform(dict(alpha_value=1), second_argument_name_long)\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(50);
            config.rules.reflow_calls.enabled = false;
        },
        "value = transform(dict(alpha_value=1), second_argument_name_long)\n"
    )]
    #[case::dict_call_behind_an_earlier_pair(
        "first = (alpha)\nvalue = (\n    dict(alpha_value=1, beta_value=2, gamma_value=3)\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(40),
        "first = alpha\nvalue = dict(alpha_value=1, beta_value=2, gamma_value=3)\n"
    )]
    fn a_forecast_reading_each_layout_setting_opens_on_the_assignment_row(
        #[case] src: &str,
        #[case] configure: fn(&mut Config),
        #[case] expected: &str,
    ) {
        assert_eq!(configured(src, configure), expected);
    }

    #[rstest]
    #[case::one_row_dict_call_past_the_budget_with_explode_off(
        "value = (\n    dict(alpha_value_long_name=1, delta_value_long_name=2, epsilon_value_name_xxxxx=3)\n)\n",
        |config: &mut Config| config.rules.reflow_collections.explode = false
    )]
    #[case::call_an_fstring_rewrite_fits_on_the_indented_row(
        "value = (\n    compute_something(alpha_argument, delta_argument, epsilon_argument_xy, \"%s\" % (name_x,))\n)\n",
        |config: &mut Config| config.target_version = Some(PythonVersion::PY310)
    )]
    #[case::one_row_dict_call_with_reflow_collections_off(
        "value = (\n    dict(alpha_value_long_name=1, delta_value_long_name=2, epsilon_value_name_xxxxx=3)\n)\n",
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::dict_call_whose_literal_row_outruns_the_budget_with_reflow_collections_off(
        &format!("value_{0} = (\n    dict(\n        alpha=1,\n        delta=2\n    )\n)\n", "k".repeat(70)),
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::one_row_call_on_a_dict_with_both_reflow_rules_off(
        "value = (\n    {\"alpha\": 1, \"beta\": 2, \"gamma\": 3}.get(key_argument, default_value)\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
            config.rules.reflow_collections.enabled = false;
        }
    )]
    #[case::one_row_call_on_a_dict_inside_the_budget_once_its_rewrite_lands_with_reflow_calls_off(
        "value = (\n    transform({\"alpha\": list(), \"beta\": 2})\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(40);
            config.rules.reflow_calls.enabled = false;
        }
    )]
    #[case::dict_call_whose_first_literal_row_outruns_the_budget_with_reflow_collections_off(
        &format!("value_{0} = (\n    dict(\n        alpha_value_long_name=1,\n        b=2\n    )\n)\n", "k".repeat(60)),
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::one_row_call_on_a_dict_inside_the_budget_once_its_padding_strips_with_reflow_calls_off(
        "value = (\n    transform_values( {\"alpha\": 1, \"beta\": 22} )\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(46);
            config.rules.reflow_calls.enabled = false;
        }
    )]
    #[case::one_row_call_on_a_dict_inside_the_budget_once_its_padding_strips(
        "value = (\n    transform_values( {\"alpha\": 1, \"beta\": 22} )\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(46)
    )]
    #[case::call_on_a_dict_call_past_max_args_with_reflow_collections_off(
        "value = (\n    transform(dict(alpha=1, beta=2, gamma=3, delta=4))\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(62);
            config.rules.reflow_collections.enabled = false;
        }
    )]
    #[case::dict_receiver_past_the_budget_with_reflow_collections_off(
        &format!("value_{0} = (\n    {{\"alpha\": 1, \"beta\": 2}}.get(key_argument_long_enough, default_value_long_enough_xxxxxxxxxxxx)\n)\n", "k".repeat(52)),
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::spanning_dict_call_of_pairs_inside_the_budget_with_explode_off(
        "value = (\n    dict(\n        [(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)]\n    )\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(54);
            config.rules.reflow_collections.explode = false;
        }
    )]
    #[case::spanning_call_on_a_dict_comprehension_call_past_the_budget_with_reflow_calls_off(
        "value = (\n    transform(dict((key, val.attr) for key, val in some_items), second_argument,\n        third)\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(60);
            config.rules.reflow_calls.enabled = false;
        }
    )]
    #[case::spanning_call_on_a_dict_call_past_the_budget_with_both_reflow_rules_off(
        "value = (\n    transform(dict(alpha=1, beta=2), second_argument_name,\n        third)\n)\n",
        |config: &mut Config| {
            config.code_line_length = NonZeroUsize::new(50);
            config.rules.reflow_calls.enabled = false;
            config.rules.reflow_collections.enabled = false;
        }
    )]
    #[case::call_on_a_dict_comprehension_call_past_the_budget(
        "value = (\n    dict((key, val.attr) for key, val in some_items_name).get(argument_x, default_value)\n)\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(55)
    )]
    #[case::fractured_call_under_a_return_one_column_short(
        "def f():\n    return(\n        compute(alpha,\n            beta)\n    )\n",
        |config: &mut Config| config.code_line_length = NonZeroUsize::new(30)
    )]
    #[case::fractured_list_comprehension_with_reflow_collections_off(
        "value = (\n    [item.name\n     for item in items]\n)\n",
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::call_on_a_fractured_dict_with_reflow_collections_off(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}.get(key)\n)\n",
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::call_without_arguments_on_a_fractured_dict_with_reflow_collections_off(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}.items()\n)\n",
        |config: &mut Config| config.rules.reflow_collections.enabled = false
    )]
    #[case::call_without_arguments_on_a_fractured_dict_past_the_budget_with_explode_off(
        "value = (\n    {\"alpha_key_name\": alpha_value_xxxxxxxx,\n     \"delta_key_name\": delta_value_xxxxxxxxxxxx}.items()\n)\n",
        |config: &mut Config| config.rules.reflow_collections.explode = false
    )]
    fn a_forecast_reading_each_layout_setting_holds_the_pair(
        #[case] src: &str,
        #[case] configure: fn(&mut Config),
    ) {
        assert_eq!(configured(src, configure), src);
    }

    #[rstest]
    fn a_forecast_value_opens_wherever_its_opened_row_fits_and_settles_in_one_run(
        #[values(
            "dict(**options_mapping)",
            "dict(base_mapping, alpha_value=1)",
            "dict(zip(key_names_list, value_names_list))",
            "dict(alpha=1, beta=2)",
            "dict(alpha_value=1, beta_value=2, gamma_value=3)",
            "dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4)",
            "dict(alpha_value_long_name=1, delta_value_long_name=2, epsilon_value_name_xxxxx=3)",
            "dict(alpha_value=1, beta_value=2, delta_value=3, epsilon_value=4, gamma_value=5, zeta_value=6)",
            "dict(alpha_value=dict(beta_value=1, gamma_value=2))",
            "dict(alpha_value=dict(beta_value=1, gamma_value=2), delta_value=3)",
            "dict(alpha_value=dict(beta_value=1, gamma_value=2, delta_value=3, epsilon_value=4))",
            "dict(alpha_value={\"x\": 1, \"y\": 2, \"z\": 3, \"w\": 4})",
            "dict(alpha_value={\"x\": 1, \"y\": 2})",
            "dict(alpha_value=[first_item, second_item, third_item])",
            "dict([(\"alpha\", 1), (\"beta\", 2)])",
            "dict([(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)])",
            "dict(((\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3)))",
            "dict({\"alpha\": 1, \"beta\": 2})",
            "dict({\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4})",
            "dict((key, val.attr) for key, val in some_items_name)",
            "dict((key, val.attr) for key, val in some_items_name).get(argument_x, default_value)",
            "dict({key: val for key, val in some_items_name}).get(argument_x, default_value)",
            "dict((k, v) for k, v in some_items_name).get(argument_x, default_value)",
            "dict([(key, val) for key, val in some_items_name])",
            "dict()",
            "dict(\n    [(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)]\n)",
            "list(item for item in some_items_name)",
            "list(item for item in some_items_name).index(argument_x, default_value)",
            "set(item for item in some_items_name).union(argument_x, default_value)",
            "list([item for item in some_items_name]).index(argument_x, default_value)",
            "list([alpha_value, beta_value, gamma_value])",
            "list((alpha_value, beta_value, gamma_value))",
            "set([alpha_value, beta_value, gamma_value])",
            "set([alpha_value, beta_value, gamma_value]).union(argument_x, default_value)",
            "list([item.attribute for item in some_items_name if item.enabled_flag])",
            "set(item.attribute for item in some_items_name if item.enabled_flag)",
            "sorted(list(item for item in some_items_name))",
            "list()",
            "transform(dict(alpha=1, beta=2))",
            "transform(dict(alpha_value=1, beta_value=2))",
            "transform(dict(alpha_value=1), second_argument_name_long)",
            "transform(dict(alpha=1, beta=2, gamma=3, delta=4))",
            "transform(dict([(\"alpha\", 1), (\"beta\", 2), (\"gamma\", 3), (\"delta\", 4)]))",
            "transform(dict(alpha=1, beta=2), second_argument_name, third_argument)",
            "transform(dict((key, val.attr) for key, val in some_items), second_argument)",
            "transform(list(item for item in some_items_name), second_argument_name)",
            "transform(set([alpha_value, beta_value]), second_argument_name_long)",
            "transform(dict(alpha_value=dict(beta_value=1, gamma_value=2)))",
            "transform(\n    dict(alpha=1, beta=2),\n    second_argument_name,\n)",
            "[dict(alpha=1, beta=2), dict(gamma=3, delta=4)]",
            "{\"alpha\": dict(beta_value=1, gamma_value=2), \"delta\": 3}",
            "{\"alpha\": list(item for item in some_items_name), \"delta\": 3}",
            "{\"alpha\": 1, \"beta\": 2, \"gamma\": 3}.get(key_argument, default_value)",
            "{\"alpha_value_name_xxxxxxx\": 1, \"delta_value_name_xxxxxxxx\": 2,\n \"epsilon_value_name\": 3}",
            "transform_values({\"alpha\": 1, \"beta\": 22})",
            "transform_values( {\"alpha\": 1, \"beta\": 22} )",
            "transform({\"alpha\": 1, \"beta\": 2222})",
            "transform({\"alpha\": list(), \"beta\": 2})",
            "compute_something(alpha_argument, delta_argument, epsilon_argument, gamma_argument_x)",
            "loop.call_later(timeout_argument, self._handle_timeout_argument, other_argument_xxxx)",
            "compute(\n    a,\n    b,\n)",
            "factory(\n    alpha,\n    beta,\n)()",
            "handlers[lookup(alpha_argument, beta_argument, gamma_argument, delta_argument_xxxxx)]()",
            "(lambda: compute(alpha_argument, beta_argument, gamma_argument, delta_argumentxxxx))()",
            "{\n    \"a\": 1,\n    \"b\": 2,\n}",
            "[\n    first,\n    second,\n]",
            "compute(alpha).transform_everything_now(\n    beta,\n    gamma,\n)",
            "compute(alpha, beta)",
            "dict(alpha_value=1, beta_value=2).get(argument_x, default_value)",
            "transform(list([alpha_value, beta_value]), set(item for item in some_items_name))",
            "dict(alpha_value=list(item for item in some_items_name), beta_value=set([first, second]))",
            "dict(alpha=dict(beta=dict(gamma=1, delta=2), epsilon=3), zeta=4)",
            "compute_values(dict(alpha=1, beta=2), dict(gamma=3, delta=4), list(item for item in items))"
        )]
        value: &str,
        #[values(false, true)] returns: bool,
        #[values(false, true)] reflows_calls: bool,
    ) {
        let (head, closer) = if returns {
            ("    return (", "    )")
        } else {
            ("value = (", ")")
        };
        for (width, text) in settled_at_every_width(value, returns, reflows_calls) {
            let mut rows = text.lines().skip_while(|row| *row != head).skip(1);
            let held = rows
                .next()
                .filter(|_| rows.next().is_some_and(|row| row != closer));
            assert!(
                held.is_none_or(|first| head.len() - 1 + first.trim_start().len() > width),
                "width {width} holds the pair around rows whose opened row fits:\n{text}",
            );
        }
    }

    #[rstest]
    fn a_forecast_value_left_paired_settles_in_one_run(
        #[values(
            "[[\n    alpha_value_name\n], beta_value_name_long]",
            "[x for x in fetch(\n)].count(key)",
            "dict(\n)[alpha,\n beta].get(key)",
            "compute([item.name for item in fetch(\n)], key_argument)",
            "table[lookup([\n    alpha\n], beta)].get(key)",
            "[item.name for item in fetch(\n) if item.compute(\n)].count(key)",
            "[item.name for item in fetch(\n) if item.compute(\n)].count(key_argument_name)",
            "compute([item.name for item in fetch(\n) if item.enabled], key_argument)",
            "[x.name for x in fetch(\n    alpha, beta\n)].count(key_argument_name)",
            "[x.name for x in fetch(\n)].run(a=1, b=2, c=3, d=4)",
            "table[lookup([\n    alpha\n], beta)].items()",
            "{\"alpha\": [\n    alpha\n], \"beta\": 2, \"gamma\": 3, \"delta\": 4}",
            "[(x.name) for x in (fetch(\n))].count(key)",
            "[(x.name)\n for x in (items)]",
            "{\n    \"alpha\": lookup_the_value(item.name\n    for item in key)}",
            "compute((alpha_value),\n    table[(beta_value)])",
            "dict(\n    alpha=1,  # first entry\n    delta=2,\n)",
            "tuple([alpha_value, beta_value, gamma_value])",
            "tuple([alpha_value, beta_value, gamma_value, delta_value, epsilon_value, zeta_value])",
            "compute(dict(alpha=1, beta=2)).result_value",
            "handlers[dict(alpha=1, beta=2)[\"alpha\"]]",
            "{\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4}.items()",
            "[compute(\n    a,\n    b,\n)]",
            "list(item for item in some_items_name)[index_value]",
            "dict(alpha_value=1, beta_value=2) | dict(gamma_value=3, delta_value=4)"
        )]
        value: &str,
        #[values(false, true)] returns: bool,
        #[values(false, true)] reflows_calls: bool,
    ) {
        settled_at_every_width(value, returns, reflows_calls);
    }

    #[rstest]
    #[case::fractured_call(
        "third = (\n    compute(a,\n        b)\n)\n",
        "third = compute(a, b)\n"
    )]
    #[case::fractured_dict(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}\n)\n",
        "value = {\"alpha\": 1, \"delta\": 2}\n"
    )]
    #[case::fractured_set_comprehension(
        "value = (\n    {item\n     for item in range(10)}\n)\n",
        "value = set(range(10))\n"
    )]
    #[case::fractured_list_comprehension(
        "value = (\n    [item.name\n     for item in items]\n)\n",
        "value = [item.name for item in items]\n"
    )]
    #[case::fractured_subscript(
        "value = (\n    table[alpha,\n          beta]\n)\n",
        "value = table[alpha, beta]\n"
    )]
    #[case::items_call_on_a_fractured_dict_past_the_budget(
        "def f():\n    pairs = (\n        {\"alpha_key_name\": alpha_value_xxxxxxxx,\n         \"delta_key_name\": delta_value_xxxxxx}.items()\n    )\n    return pairs\n",
        "def f():\n    pairs = {\n        \"alpha_key_name\" : alpha_value_xxxxxxxx,\n        \"delta_key_name\" : delta_value_xxxxxx\n    }.items()\n    return pairs\n"
    )]
    #[case::items_call_on_an_item_of_a_dict_past_the_budget(
        "value = (\n    {\"alpha_key_name\": alpha_value_xxxxxxxx, \"delta_key_name\": delta_value_xxxxxx}[key].items()\n)\n",
        "value = {\n    \"alpha_key_name\" : alpha_value_xxxxxxxx,\n    \"delta_key_name\" : delta_value_xxxxxx\n}[key].items()\n"
    )]
    #[case::items_call_on_a_dict_past_the_budget(
        "def f():\n    pairs = (\n        {\"alpha_key_name\": alpha_value_xxxxxxxx, \"delta_key_name\": delta_value_xxxxxx}.items()\n    )\n    return pairs\n",
        "def f():\n    pairs = {\n        \"alpha_key_name\" : alpha_value_xxxxxxxx,\n        \"delta_key_name\" : delta_value_xxxxxx\n    }.items()\n    return pairs\n"
    )]
    #[case::fractured_set_comprehension_whose_rewrite_lands_at_the_budget(
        &format!("value_{0} = (\n    {{item\n     for item in range(10)}}\n)\n", "k".repeat(65)),
        &format!("value_{0} = set(range(10))\n", "k".repeat(65))
    )]
    #[case::fractured_dict_rejoining_at_the_budget_under_a_long_target(
        &format!("value_{0} = (\n    {{\"alpha\": 1,\n     \"delta\": 2}}\n)\n", "k".repeat(55)),
        &format!("value_{0} = {{\"alpha\": 1, \"delta\": 2}}\n", "k".repeat(55))
    )]
    #[case::fractured_call_reflow_calls_condenses_to_the_budget(
        &format!("value_{0} = (\n    compute(alpha,\n        beta)\n)\n", "k".repeat(59)),
        &format!("value_{0} = compute(alpha, beta)\n", "k".repeat(59))
    )]
    #[case::fractured_dict_call_of_pairs_whose_rewrite_lands_on_one_row(
        "value = (\n    dict([(alpha,\n           beta)])\n)\n",
        "value = {alpha: beta}\n"
    )]
    #[case::call_on_a_one_entry_list_written_across_rows(
        "value = (\n    [\n        alpha\n    ].count(key)\n)\n",
        "value = [alpha].count(key)\n"
    )]
    #[case::call_on_a_fractured_dict(
        "value = (\n    {\"alpha\": 1,\n     \"delta\": 2}.get(key)\n)\n",
        "value = {\"alpha\": 1, \"delta\": 2}.get(key)\n"
    )]
    #[case::fractured_call_holding_a_call_ahead_of_its_break(
        "value = (\n    compute(inner(a),\n        b)\n)\n",
        "value = compute(inner(a), b)\n"
    )]
    #[case::call_on_a_subscript_slice_written_across_rows(
        "value = (\n    table[alpha,\n          beta].get(key)\n)\n",
        "value = table[alpha, beta].get(key)\n"
    )]
    #[case::call_on_a_list_comprehension_written_across_rows(
        "value = ([item.name\n for item in items].count(key))\n",
        "value = [item.name for item in items].count(key)\n"
    )]
    #[case::call_holding_a_dict_comprehension_written_across_rows(
        "value = (\n    compute({item.name: item\n             for item in items}, key)\n)\n",
        "value = compute({item.name: item for item in items}, key)\n"
    )]
    #[case::call_holding_a_generator_written_across_rows(
        "value = (\n    compute(item.name\n            for item in items)\n)\n",
        "value = compute(item.name for item in items)\n"
    )]
    #[case::call_on_a_subscript_of_a_call_with_empty_brackets_across_rows(
        "value = (\n    obj.compute(\n    )[alpha].get(key)\n)\n",
        "value = obj.compute()[alpha].get(key)\n"
    )]
    #[case::call_on_a_comprehension_over_a_call_with_empty_brackets_across_rows(
        "value = (\n    [x.name for x in fetch(\n    )].count(key)\n)\n",
        "value = [x.name for x in fetch()].count(key)\n"
    )]
    #[case::call_on_a_simplified_comprehension_over_a_call_with_empty_brackets_across_rows(
        "value = (\n    [x for x in fetch(\n    )].count(key)\n)\n",
        "value = list(fetch()).count(key)\n"
    )]
    #[case::call_on_a_fractured_subscript_of_a_call_with_empty_brackets_across_rows(
        "value = (\n    dict(\n    )[alpha,\n       beta].get(key)\n)\n",
        "value = {}[alpha, beta].get(key)\n"
    )]
    #[case::comprehension_over_a_call_with_empty_brackets_across_rows(
        "value = (\n    [x.name for x in fetch(\n    )]\n)\n",
        "value = [x.name for x in fetch()]\n"
    )]
    #[case::subscript_of_a_call_with_empty_brackets_across_rows(
        "value = (\n    obj.compute(\n    )[alpha]\n)\n",
        "value = obj.compute()[alpha]\n"
    )]
    #[case::call_holding_a_one_entry_list_whose_closer_row_holds_more_arguments(
        "value = (\n    compute([\n        alpha\n    ], key)\n)\n",
        "value = compute([alpha], key)\n"
    )]
    #[case::call_without_arguments_on_a_subscript_of_a_one_entry_list(
        "value = (\n    table[[\n        alpha\n    ], beta].items()\n)\n",
        "value = table[[alpha], beta].items()\n"
    )]
    #[case::count_exploded_call_on_a_comprehension_over_a_call_with_empty_brackets_across_rows(
        "value = (\n    [x.name for x in fetch(\n    )].run(a=1, b=2, c=3, d=4)\n)\n",
        "value = [x.name for x in fetch()].run(\n    a = 1,\n    b = 2,\n    c = 3,\n    d = 4\n)\n"
    )]
    #[case::fractured_count_exploded_call_on_a_comprehension_over_a_call_with_empty_brackets(
        "value = (\n    [x.name for x in fetch(\n    )].run(a=1, b=2,\n        c=3, d=4)\n)\n",
        "value = [x.name for x in fetch()].run(\n    a = 1,\n    b = 2,\n    c = 3,\n    d = 4\n)\n"
    )]
    #[case::call_on_a_comprehension_shedding_nested_pairs_to_the_budget(
        &format!("value_{0} = (\n    [(x.name) for x in (fetch(\n    ))].count(key)\n)\n", "k".repeat(43)),
        &format!("value_{0} = [x.name for x in fetch()].count(key)\n", "k".repeat(43))
    )]
    #[case::fractured_comprehension_shedding_nested_pairs_to_the_budget(
        &format!("value_{0} = (\n    [(x.name)\n     for x in (items)]\n)\n", "k".repeat(56)),
        &format!("value_{0} = [x.name for x in items]\n", "k".repeat(56))
    )]
    #[case::dict_call_with_empty_brackets_across_rows(
        "value = (\n    dict(\n    )\n)\n",
        "value = {}\n"
    )]
    #[case::empty_dict_across_rows("value = (\n    {\n    }\n)\n", "value = {}\n")]
    fn a_pair_around_an_interior_a_later_rule_joins_settles_in_one_run(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let pipeline = Pipeline::with_defaults(&Config::default());
        let once = pipeline.format(parse(src)).expect("the first run formats");
        assert_eq!(once.text(), expected);
        let twice = pipeline
            .format(parse(once.text()))
            .expect("the second run formats");
        assert_eq!(twice.text(), expected);
    }

    #[test]
    fn a_dict_call_on_a_dict_past_the_entry_cap_opens_at_every_width_its_pair_fits() {
        let src =
            "value = (\n    dict({\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4})\n)\n";
        for width in 51..=54 {
            assert_eq!(
                configured(src, |config| config.code_line_length =
                    NonZeroUsize::new(width)),
                "value = dict({\"alpha\": 1, \"beta\": 2, \"gamma\": 3, \"delta\": 4})\n",
                "width {width}",
            );
        }
    }

    #[test]
    fn a_call_on_a_dict_comprehension_call_holds_wherever_its_rewritten_row_runs_past_the_budget() {
        let src = "value = (\n    dict((key, val.attr) for key, val in some_items_name).get(argument_x, default_value)\n)\n";
        for width in 30..=59 {
            assert_eq!(
                configured(src, |config| config.code_line_length =
                    NonZeroUsize::new(width)),
                src,
                "width {width}",
            );
        }
    }

    #[test]
    fn a_value_simplify_comprehensions_rewrites_over_two_runs_is_measured_as_its_first() {
        let written = paired(
            "list(item for item in dict(alpha_value=1, beta_value=2, gamma_value=3))",
            false,
        );
        let landed = paired(
            "list(item for item in {\"alpha_value\": 1, \"beta_value\": 2, \"gamma_value\": 3})",
            false,
        );
        for width in 30..=88 {
            let mut config = Config {
                code_line_length: NonZeroUsize::new(width),
                ..Config::default()
            };
            let rewritten = Pipeline::with_defaults(&config)
                .format(parse(&written))
                .expect("the rewritten value formats");
            config.rules.simplify_comprehensions.enabled = false;
            let literal = Pipeline::with_defaults(&config)
                .format(parse(&landed))
                .expect("the landed value formats");
            assert_eq!(rewritten.text(), literal.text(), "width {width}");
        }
    }

    #[test]
    fn a_module_of_paired_dict_calls_measures_every_rewrite_against_one_reparse() {
        let src: String = (0..PAIRED_DICT_CALLS)
            .map(|index| {
                format!(
                    "value_{index} = (\n    dict(alpha_value={index}, beta_value=2, gamma_value=3)\n)\n"
                )
            })
            .collect();
        let config = Config {
            code_line_length: NonZeroUsize::new(50),
            ..Config::default()
        };
        let source = parse(&src);
        let rule = ReflowParentheses::from_config(&config);
        let started = Instant::now();
        let groups = rule.apply(&source);
        let elapsed = started.elapsed();
        assert_eq!(groups.len(), PAIRED_DICT_CALLS);
        assert!(elapsed < PAIRED_DICT_CALLS_BOUND, "{elapsed:?}");
    }
}
