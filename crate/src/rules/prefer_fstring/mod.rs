//! Converts printf-style `%` interpolation and `str.format()` calls to
//! f-strings, holding every template whose two forms would not render
//! alike, every value a replacement field cannot carry, and every
//! rewrite that would run its line past the budget.

use ruff_diagnostics::Edit;
use ruff_python_ast::{Expr, PythonVersion};
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        edit::{narrowed_replacement, singleton_groups},
        inline::{end_column, opening_width},
        walk::filter_map_over_exprs,
    },
    rule::{Rule, RuleId},
    source::Source,
};

mod field;
mod format_call;
mod literal;
mod percent;
mod spec;

/// The release f-strings landed in.
const FSTRING_FLOOR: PythonVersion = PythonVersion { major: 3, minor: 6 };

pub(crate) struct PreferFstring {
    budget: usize,
    percent: bool,
    str_format: bool,
}

impl PreferFstring {
    pub(crate) fn from_config(config: &Config) -> Self {
        let facets = &config.rules.prefer_fstring;
        let targets = config
            .target_version
            .is_some_and(|target| target >= FSTRING_FLOOR);
        Self {
            budget: config.code_width(),
            percent: facets.rewrite_percent && targets,
            str_format: facets.rewrite_str_format && targets,
        }
    }

    /// The edit rewriting `expr` as an f-string, `None` when the shape
    /// declines, when its facet is off, when the rewritten line runs
    /// past the budget, and when a directive suppresses the line.
    fn edit(&self, source: &Source, expr: &Expr) -> Option<Edit> {
        let rewrite = match expr {
            Expr::BinOp(binop) if self.percent => percent::rewritten(source, binop),
            Expr::Call(call) if self.str_format => format_call::rewritten(source, call),
            _ => None,
        }?;
        let span = expr.range();
        let rewrite = spaced(source, span, rewrite);
        if !self.fits(source, span, &rewrite) {
            return None;
        }
        let edit = narrowed_replacement(source, span, rewrite)?;
        (!source.suppression_map().suppresses(&edit, Self::SLUG)).then_some(edit)
    }

    /// True when every physical line `rewrite` lands on stays inside
    /// the budget. Only the first line carries the text preceding
    /// `span` and only the last carries what follows it.
    fn fits(&self, source: &Source, span: TextRange, rewrite: &str) -> bool {
        let after = &source.text()[span.end().to_usize()..];
        let tail = after.split_once('\n').map_or(after, |(head, _)| head);
        let inner = rewrite.lines().skip(1);
        !source.column_overflows(span.start(), opening_width(rewrite), self.budget)
            && end_column(rewrite, source.column_of(span.start())) + tail.width() <= self.budget
            && inner
                .clone()
                .zip(inner.skip(1))
                .all(|(line, _)| line.width() <= self.budget)
    }
}

impl Rule for PreferFstring {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        if !self.percent && !self.str_format {
            return Vec::new();
        }
        singleton_groups(filter_map_over_exprs(&source.ast().body, |expr| {
            self.edit(source, expr)
        }))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// `rewrite` behind a space wherever a keyword abuts the template, so
/// `return"{}".format(x)` does not settle as `returnf"{x}"`.
fn spaced(source: &Source, span: TextRange, rewrite: String) -> String {
    let abuts = source.text()[TextRange::up_to(span.start())]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_');
    if abuts {
        format!(" {rewrite}")
    } else {
        rewrite
    }
}

#[cfg(test)]
mod tests {
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
    fn a_keyword_abutting_the_template_keeps_its_separating_space() {
        assert_eq!(
            run("def f(x):\n    return\"%s\" % (x,)\n"),
            "def f(x):\n    return f\"{x}\"\n"
        );
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
    fn no_target_version_holds_every_template() {
        let source = parse("x = \"%s\" % (a,)\n");
        assert!(rule(None).apply(&source).is_empty());
    }

    #[test]
    fn a_target_below_the_floor_holds_every_template() {
        let source = parse("x = \"%s\" % (a,)\n");
        let below = PythonVersion { major: 3, minor: 5 };
        assert!(rule(Some(below)).apply(&source).is_empty());
    }

    #[test]
    fn a_convertible_template_inside_another_leaves_the_outer_alone() {
        assert_eq!(
            run("x = \"%s\" % (\"%s\" % (a,),)\n"),
            "x = \"%s\" % (f\"{a}\",)\n"
        );
    }
}
