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
//! landing across rows comes off, that construct opening on the
//! statement's row, unless that row then runs past the budget the
//! pair's own row fits or the construct's rows cannot hang from the
//! statement's indent. An over-budget boolean chain or
//! conditional forming a test, a `return` value, an assignment value, or
//! the last element of a bare tuple standing as one gains a pair the
//! break adds. Each row is measured as the later rules shortening it
//! leave it.
//! `plan` finds the pairs, `chain` divides an interior into operands,
//! `measure` answers whether each direction fits, and `render` builds
//! the broken text.

use std::cell::RefCell;

use ruff_diagnostics::Edit;
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
        let calls = if self.reflows_calls {
            outermost_calls(source)
        } else {
            Vec::new()
        };
        let stranded = source.stranded_padding(self.stranding);
        let simplified = if self.simplifies_comprehensions {
            prepared_groups(&SimplifyComprehensions, source)
        } else {
            Vec::new()
        };
        let fstrings = source.fstring_rewrites(self.fstrings);
        let rewrites = self.rewrites(source, &simplified, &fstrings);
        let padding = padding::beside(&stranded, &rewrites);
        let reservations = source.columns(&self.reservations);
        let targets = module_call_params(source);
        let mut shedder = Shedder {
            breaks: RefCell::default(),
            calls,
            code_line_length: self.code_line_length,
            edits: Vec::new(),
            folds: Vec::new(),
            one_row: self.one_row.against(&targets).forecasting(&fstrings),
            padding: &padding,
            reservations: &reservations,
            simplified: &simplified,
            source,
        };
        shedder.shed(&candidates);
        singleton_groups(outermost(shedder.edits))
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
/// `simplified` the fix groups `simplify-comprehensions` emits, and
/// `breaks` holds, for the candidate under test, whether each later pair
/// on its row breaks.
struct Shedder<'a> {
    breaks: RefCell<FxHashMap<TextRange, bool>>,
    calls: Vec<TextRange>,
    code_line_length: usize,
    edits: Vec<Edit>,
    folds: Vec<TextRange>,
    one_row: one_row::Settings<'a>,
    padding: &'a [Edit],
    reservations: &'a reserve::Columns,
    simplified: &'a [Vec<Edit>],
    source: &'a Source,
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
    /// assignment that is one bracketed construct landing across rows
    /// opens that bracket on the statement's row, and any other pair holds, whereas
    /// a pair the break adds earns nothing.
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
    use rstest::rstest;
    use ruff_python_ast::PythonVersion;

    use super::*;
    use crate::testing::{applied_text, parse};

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
    #[case::fractured_call_rejoining_inside("value = (\n    compute(a,\n        b)\n)\n", true)]
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
        &format!("value_{0} = (\n    dict(\n        alpha=1,\n        delta=2\n    )\n)\n", "k".repeat(70)),
        |config: &mut Config| config.rules.reflow_collections.enabled = false,
        &format!("value_{0} = dict(\n    alpha=1,\n    delta=2\n)\n", "k".repeat(70))
    )]
    #[case::spanning_dict_call_past_the_budget_with_explode_off(
        "value = (\n    dict(\n        alpha_value_long_name=1,\n        delta_value_long_name=2,\n        epsilon_value_name_xxxxx=3\n    )\n)\n",
        |config: &mut Config| config.rules.reflow_collections.explode = false,
        "value = dict(\n    alpha_value_long_name=1,\n    delta_value_long_name=2,\n    epsilon_value_name_xxxxx=3\n)\n"
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
    fn a_forecast_reading_each_layout_setting_holds_the_pair(
        #[case] src: &str,
        #[case] configure: fn(&mut Config),
    ) {
        assert_eq!(configured(src, configure), src);
    }
}
