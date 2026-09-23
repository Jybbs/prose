//! Converts printf-style `%` interpolation and `str.format()` calls to
//! f-strings, holding every template whose two forms would not render
//! alike, every value a replacement field cannot carry, every construct
//! carrying a comment, and every rewrite whose rows run past the budget
//! once every rewrite on them lands and `strip-stranded-padding` clears
//! their padding.

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, PythonVersion};
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        edit::{narrowed_replacement, padded, singleton_groups},
        fracture::outermost,
        inline::rows_within,
        padding::Stranding,
        walk::{Descent, filter_map_over_parented_exprs},
    },
    rules::{Rule, RuleId},
    source::Source,
};

mod field;
mod format_call;
mod literal;
mod percent;
mod spec;

/// The release f-strings landed in.
const FSTRING_FLOOR: PythonVersion = PythonVersion { major: 3, minor: 6 };

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PreferFstring {
    code_line_length: usize,
    percent: bool,
    str_format: bool,
    stranding: Stranding,
}

impl PreferFstring {
    pub(crate) const MESSAGE: &'static str =
        "convert `%` or `str.format()` interpolation to an f-string";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) const PRESERVES_TREE: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let facets = &config.rules.prefer_fstring;
        let targets = facets.enabled
            && config
                .target_version
                .is_some_and(|target| target >= FSTRING_FLOOR);
        Self {
            code_line_length: config.code_width(),
            percent: facets.rewrite_percent && targets,
            str_format: facets.rewrite_str_format && targets,
            stranding: config.stranded_padding(),
        }
    }

    /// The edit rewriting `expr` as an f-string beside the span it
    /// replaces, `None` when the shape declines, when its facet is off,
    /// and when a comment sits inside the construct. The replaced span
    /// takes in a grouping pair the f-string leaves redundant, holding
    /// that pair wherever a comment sits inside it.
    fn rewrite(
        &self,
        source: &Source,
        expr: &Expr,
        parent: AnyNodeRef,
    ) -> Option<(TextRange, Edit)> {
        let text = match expr {
            Expr::BinOp(binop) if self.percent => percent::rewritten(source, binop),
            Expr::Call(call) if self.str_format => format_call::rewritten(source, call),
            _ => None,
        }?;
        if !source
            .comment_ranges()
            .comments_in_range(expr.range())
            .is_empty()
        {
            return None;
        }
        let grouped = source.paren_aware_range(expr.into(), parent);
        let span = if source.intersects_comment(grouped) {
            expr.range()
        } else {
            grouped
        };
        let text = padded(source, span.start(), text);
        narrowed_replacement(source, span, text).map(|edit| (span, edit))
    }

    /// Each rewrite over `source` beside the span it replaces, ascending,
    /// none where both facets are off.
    fn rewrites(&self, source: &Source) -> Vec<(TextRange, Edit)> {
        if !self.percent && !self.str_format {
            return Vec::new();
        }
        let mut rewrites =
            filter_map_over_parented_exprs(source.ast(), Descent::Over, |expr, parent| {
                self.rewrite(source, expr, parent)
            });
        rewrites.sort_unstable_by_key(|(_, edit)| edit.start());
        rewrites
    }

    /// Every edit [`apply`](Rule::apply) would emit over `source` were
    /// no row held to the budget, ascending, leaving out each rewrite
    /// whose replaced span covers a line break. A rule measuring ahead
    /// of this one reads a `%` or `str.format()` interpolation at the
    /// width of the f-string it becomes.
    pub(crate) fn forecast(&self, source: &Source) -> Vec<Edit> {
        self.rewrites(source)
            .into_iter()
            .filter(|(span, _)| !source.contains_line_break(*span))
            .map(|(_, edit)| edit)
            .collect()
    }
}

impl Rule for PreferFstring {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let rewrites = self.rewrites(source);
        let landed = outermost(
            source
                .stranded_padding(self.stranding)
                .iter()
                .chain(rewrites.iter().map(|(_, edit)| edit))
                .cloned()
                .collect(),
        );
        singleton_groups(
            rewrites
                .into_iter()
                .filter(|(span, _)| rows_within(source, *span, &landed, self.code_line_length))
                .map(|(_, edit)| edit),
        )
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use rstest::rstest;

    use super::*;
    use crate::testing::{applied_text, parse};

    fn rule(version: Option<PythonVersion>) -> PreferFstring {
        PreferFstring::from_config(&Config {
            target_version: version,
            ..Config::default()
        })
    }

    /// `src` with every rewrite the rule emits applied under 3.10.
    fn run(src: &str) -> String {
        let source = parse(src);
        let edits = rule(Some(PythonVersion::PY310)).apply(&source).concat();
        applied_text(&source, edits)
    }

    #[test]
    fn a_convertible_template_inside_another_leaves_the_outer_alone() {
        assert_eq!(
            run("x = \"%s\" % (\"%s\" % (a,),)\n"),
            "x = \"%s\" % (f\"{a}\",)\n"
        );
    }

    #[test]
    fn a_disabled_rule_forecasts_nothing() {
        let mut config = Config {
            target_version: Some(PythonVersion::PY310),
            ..Config::default()
        };
        config.rules.prefer_fstring.enabled = false;
        let source = parse("x = \"%s\" % (a,)\n");
        assert!(config.fstrings().forecast(&source).is_empty());
    }

    #[test]
    fn a_keyword_abutting_the_template_keeps_its_separating_space() {
        assert_eq!(
            run("def f(x):\n    return\"%s\" % (x,)\n"),
            "def f(x):\n    return f\"{x}\"\n"
        );
    }

    #[rstest]
    #[case::past_the_budget(18, false)]
    #[case::at_the_budget(19, true)]
    #[case::one_column_under(20, true)]
    #[case::two_columns_under(21, true)]
    fn a_rewrite_reads_its_row_with_the_padding_cleared(
        #[case] width: usize,
        #[case] converts: bool,
    ) {
        let config = Config {
            code_line_length: NonZeroUsize::new(width),
            target_version: Some(PythonVersion::PY310),
            ..Config::default()
        };
        let source = parse("x = [ \"%s\" % ( alpha, ), b ]\n");
        assert_eq!(
            !PreferFstring::from_config(&config)
                .apply(&source)
                .is_empty(),
            converts
        );
    }

    #[rstest]
    #[case(None)]
    #[case(Some(PythonVersion { major: 3, minor: 5 }))]
    fn a_target_below_the_floor_holds_every_template(#[case] version: Option<PythonVersion>) {
        let source = parse("x = \"%s\" % (a,)\n");
        assert!(rule(version).apply(&source).is_empty());
    }

    #[test]
    fn both_facets_disabled_emit_no_edits() {
        let mut config = Config {
            target_version: Some(PythonVersion::PY310),
            ..Config::default()
        };
        config.rules.prefer_fstring.rewrite_str_format = false;
        config.rules.prefer_fstring.rewrite_percent = false;
        let source = parse("x = \"%s\" % (a,)\n");
        assert!(
            PreferFstring::from_config(&config)
                .apply(&source)
                .is_empty()
        );
    }

    #[test]
    fn rewrites_sharing_a_row_are_held_to_the_budget_together() {
        let config = Config {
            code_line_length: NonZeroUsize::new(30),
            target_version: Some(PythonVersion::PY310),
            ..Config::default()
        };
        let source = parse("x = [\"{}\".format(aaaa), \"{}\".format(bbbb)]\n");
        let edits = PreferFstring::from_config(&config).apply(&source).concat();
        assert_eq!(
            applied_text(&source, edits),
            "x = [f\"{aaaa}\", f\"{bbbb}\"]\n"
        );
    }

    #[test]
    fn the_forecast_keeps_a_rewrite_past_the_budget_and_leaves_out_one_across_rows() {
        let config = Config {
            code_line_length: NonZeroUsize::new(20),
            target_version: Some(PythonVersion::PY310),
            ..Config::default()
        };
        let source = parse(
            "x = \"%s\" % (a,)\ny = \"%s and %s\" % (\n    b,\n    c,\n)\nlong_name = \"%s\" % (value,)\n",
        );
        let rule = PreferFstring::from_config(&config);
        let contents = |edits: &[Edit]| -> Vec<String> {
            edits
                .iter()
                .filter_map(Edit::content)
                .map(str::to_owned)
                .collect()
        };
        assert_eq!(
            contents(&rule.forecast(&source)),
            ["f\"{a}\"", "f\"{value}\""]
        );
        assert_eq!(
            contents(&rule.apply(&source).concat()),
            ["f\"{a}\"", "f\"{b} and {c}\""]
        );
    }
}
