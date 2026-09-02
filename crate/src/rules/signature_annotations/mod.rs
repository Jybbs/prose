//! Reports a function parameter that carries no type annotation and a
//! value-returning function that carries no return annotation. `self`,
//! `cls`, `*args`, and `**kwargs` stay outside the parameter report. A
//! literal default or in-module call sites passing only literals produce
//! a display-only annotation suggestion, never auto-applied.

use ruff_diagnostics::Edit;
use ruff_python_ast::{Expr, ParameterWithDefault, Parameters, Stmt, StmtFunctionDef};
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashMap;

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::{params::first_positional, walk::filter_map_over_stmts},
    rule::{Rule, RuleId},
    source::Source,
};

mod literals;
mod signals;

use literals::{call_argument_literals, returns_value};
use signals::SignalSet;

#[derive(Debug)]
pub(crate) struct SignatureAnnotations;

impl SignatureAnnotations {
    pub(crate) const MESSAGE: &'static str = "Flag a missing parameter or return type annotation";

    pub(crate) const PRESERVES_BINDINGS: bool = true;

    pub(crate) fn from_config(_: &Config) -> Self {
        Self
    }
}

impl Rule for SignatureAnnotations {
    fn id(&self) -> RuleId {
        Self::SLUG
    }

    fn lint(&self, source: &Source) -> Vec<Diagnostic> {
        let mut walker = Walker {
            call_args: call_argument_literals(source),
            diagnostics: Vec::new(),
            rule: self.id(),
        };
        for fd in filter_map_over_stmts(&source.ast().body, Stmt::as_function_def_stmt) {
            walker.process_def(fd);
        }
        walker.diagnostics
    }
}

/// Per resolved module function (keyed by its parameters' start), the
/// call-site argument bound to each named parameter.
type CallArgs<'a> = FxHashMap<TextSize, FxHashMap<&'a str, Vec<&'a Expr>>>;

/// Emits the parameter reports and the missing-return report for each
/// function definition, reading `call_args` for the call-site arguments
/// bound to each parameter.
struct Walker<'a> {
    call_args: CallArgs<'a>,
    diagnostics: Vec<Diagnostic>,
    rule: RuleId,
}

impl Walker<'_> {
    /// Emits the parameter reports and, for a value-returning function
    /// with no return annotation, the missing-return report.
    fn process_def(&mut self, fd: &StmtFunctionDef) {
        let params: &Parameters = &fd.parameters;
        let params_start = params.start();
        let receiver = first_positional(params).map(|p| p.start());
        for param in params.iter_non_variadic_params() {
            if param.annotation().is_some() {
                continue;
            }
            if Some(param.start()) == receiver && matches!(param.name().as_str(), "self" | "cls") {
                continue;
            }
            self.report_param(param, params_start);
        }
        if fd.returns.is_none() && returns_value(fd) {
            self.diagnostics.push(Diagnostic::lint(
                self.rule,
                fd.name.range(),
                format!(
                    "`{}` returns a value but has no return type annotation",
                    fd.name.as_str(),
                ),
            ));
        }
    }

    /// Reports the unannotated `param`, attaching a display-only
    /// suggestion when its default and call-site arguments agree on a
    /// confident type.
    fn report_param(&mut self, param: &ParameterWithDefault, params_start: TextSize) {
        let name = param.name().as_str();
        let range = param.name().range();
        let mut signals = SignalSet::default();
        if let Some(default) = param.default() {
            signals.add(default);
        }
        for &arg in self
            .call_args
            .get(&params_start)
            .and_then(|bound| bound.get(name))
            .into_iter()
            .flatten()
        {
            signals.add(arg);
        }
        let base = format!("Parameter `{name}` has no type annotation");
        let diagnostic = match signals.suggestion() {
            Some(annotation) => Diagnostic::suggestion(
                self.rule,
                range,
                format!("{base}. Consider `{name}: {annotation}`"),
                Edit::insertion(format!(": {annotation}"), range.end()),
            ),
            None => Diagnostic::lint(self.rule, range, base),
        };
        self.diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Applicability;

    use super::*;
    use crate::{
        diagnostics::Severity,
        testing::{first_def, first_value, parse},
    };

    fn param_report<'a>(diagnostics: &'a [Diagnostic], name: &str) -> &'a Diagnostic {
        diagnostics
            .iter()
            .find(|d| d.message.contains(&format!("`{name}`")))
            .expect("a report for the named parameter")
    }

    fn suggestion_for(values: &[&str]) -> Option<String> {
        let mut signals = SignalSet::default();
        for value in values {
            let source = parse(&format!("_ = {value}\n"));
            signals.add(first_value(&source));
        }
        signals.suggestion()
    }

    #[test]
    fn a_confident_signal_becomes_a_display_only_suggestion() {
        let source = parse("def f(threshold=0.5):\n    return threshold\n");
        let rule = SignatureAnnotations::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let report = param_report(&diagnostics, "threshold");

        assert_eq!(report.severity, Severity::Lint);
        let fix = report.fix.as_ref().expect("display-only suggestion");
        assert_eq!(fix.applicability(), Applicability::DisplayOnly);
        assert_eq!(fix.edits()[0].content(), Some(": float"));
        assert!(report.message.ends_with("Consider `threshold: float`"));
    }

    #[test]
    fn a_keyword_only_self_is_not_treated_as_a_receiver() {
        let source = parse("def f(*, self):\n    return self\n");
        let rule = SignatureAnnotations::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let report = param_report(&diagnostics, "self");

        assert!(report.message.starts_with("Parameter"));
    }

    #[test]
    fn a_receiver_and_the_variadics_stay_unreported() {
        let source = parse("class C:\n    def m(self, *args, **kwargs):\n        return args\n");
        let rule = SignatureAnnotations::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.starts_with("Parameter"))
        );
    }

    #[test]
    fn an_unsuggested_report_carries_no_fix() {
        let source = parse("def f(opt=None):\n    return opt\n");
        let rule = SignatureAnnotations::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        let report = param_report(&diagnostics, "opt");

        assert_eq!(report.severity, Severity::Lint);
        assert!(report.fix.is_none());
    }

    #[rstest]
    #[case("def f():\n    return value\n", true)]
    #[case("def f():\n    return None\n", false)]
    #[case("def f():\n    return\n", false)]
    #[case("def f():\n    pass\n", false)]
    #[case("def f():\n    yield 1\n", false)]
    #[case("def f():\n    def inner():\n        return 1\n", false)]
    fn returns_value_counts_only_the_function_s_own_value_returns(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(returns_value(first_def(&parse(src))), expected);
    }

    #[rstest]
    #[case(&["1"], Some("int"))]
    #[case(&["1.5"], Some("float"))]
    #[case(&["1j"], Some("complex"))]
    #[case(&["\"s\""], Some("str"))]
    #[case(&["b\"s\""], Some("bytes"))]
    #[case(&["True"], Some("bool"))]
    #[case(&["-1"], Some("int"))]
    #[case(&["None"], None)]
    #[case(&["None", "\"s\""], Some("str | None"))]
    #[case(&["1", "1"], Some("int"))]
    #[case(&["1", "\"s\""], None)]
    #[case(&["compute()"], None)]
    #[case(&["1", "compute()"], None)]
    fn suggestion_folds_literal_signals_into_one_scalar_type(
        #[case] values: &[&str],
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(suggestion_for(values).as_deref(), expected);
    }
}
