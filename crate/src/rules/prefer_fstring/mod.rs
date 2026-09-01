//! Converts printf-style `%` interpolation and `str.format()` calls to
//! f-strings, holding every template whose two forms would not render
//! alike, every value a replacement field cannot carry, every construct
//! carrying a comment, and every rewrite that would run its line past
//! the budget.

use std::slice;

use ruff_diagnostics::Edit;
use ruff_python_ast::{AnyNodeRef, Expr, PythonVersion};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::{
    config::Config,
    primitives::{
        edit::{apply_inline_edits, narrowed_replacement, padded, singleton_groups},
        inline::display_width,
        walk::{Descent, filter_map_over_parented_exprs},
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

#[derive(Debug)]
pub(crate) struct PreferFstring {
    code_line_length: usize,
    percent: bool,
    str_format: bool,
}

impl PreferFstring {
    pub(crate) const MESSAGE: &'static str =
        "convert `%` or `str.format()` interpolation to an f-string";

    pub(crate) const PRESERVES_BINDINGS: bool = false;

    pub(crate) fn from_config(config: &Config) -> Self {
        let facets = &config.rules.prefer_fstring;
        let targets = config
            .target_version
            .is_some_and(|target| target >= FSTRING_FLOOR);
        Self {
            code_line_length: config.code_width(),
            percent: facets.rewrite_percent && targets,
            str_format: facets.rewrite_str_format && targets,
        }
    }

    /// The edit rewriting `expr` as an f-string, `None` when the shape
    /// declines, when its facet is off, when a comment sits inside the
    /// construct, and when the rewritten line runs past the budget. The
    /// replaced span takes in a grouping pair the f-string leaves
    /// redundant, holding that pair wherever a comment sits inside it.
    fn edit(&self, source: &Source, expr: &Expr, parent: AnyNodeRef) -> Option<Edit> {
        let rewrite = match expr {
            Expr::BinOp(binop) if self.percent => percent::rewritten(source, binop),
            Expr::Call(call) if self.str_format => format_call::rewritten(source, call),
            _ => None,
        }?;
        if source.intersects_comment(expr) {
            return None;
        }
        let grouped = source.paren_aware_range(expr.into(), parent);
        let span = if source.intersects_comment(grouped) {
            expr.range()
        } else {
            grouped
        };
        let rewrite = padded(source, span.start(), rewrite);
        let edit = narrowed_replacement(source, span, rewrite)?;
        self.fits(source, span, &edit).then_some(edit)
    }

    /// True when every physical line `edit` lands on stays inside the
    /// budget, measured over the lines `span` spans with the rewrite
    /// spliced in.
    fn fits(&self, source: &Source, span: TextRange, edit: &Edit) -> bool {
        apply_inline_edits(
            source,
            source.text().lines_range(span),
            slice::from_ref(edit),
        )
        .lines()
        .all(|line| display_width(line) <= self.code_line_length)
    }
}

impl Rule for PreferFstring {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        if !self.percent && !self.str_format {
            return Vec::new();
        }
        singleton_groups(filter_map_over_parented_exprs(
            source.ast(),
            Descent::Over,
            |expr, parent| self.edit(source, expr, parent),
        ))
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

#[cfg(test)]
mod tests {
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
    fn a_keyword_abutting_the_template_keeps_its_separating_space() {
        assert_eq!(
            run("def f(x):\n    return\"%s\" % (x,)\n"),
            "def f(x):\n    return f\"{x}\"\n"
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
}
