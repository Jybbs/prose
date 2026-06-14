//! Reports a function parameter that carries no type annotation and a
//! value-returning function that carries no return annotation. `self`,
//! `cls`, `*args`, and `**kwargs` stay outside the parameter report. A
//! literal default or in-module call sites passing only literals ride
//! as a display-only annotation suggestion, never auto-applied.

use std::collections::{BTreeSet, HashMap};

use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Expr, LiteralExpressionRef, Number, ParameterWithDefault, Parameters, Stmt, StmtFunctionDef,
    UnaryOp,
    helpers::ReturnStatementVisitor,
    statement_visitor::{StatementVisitor, walk_stmt},
    visitor::{Visitor as AstVisitor, walk_expr},
};
use ruff_text_size::{Ranged, TextSize};

use crate::{
    config::Config,
    diagnostics::Diagnostic,
    primitives::call_keywords::{keyword_args, module_call_params, resolve_call_params},
    rule::{Rule, RuleId},
    source::Source,
};

/// Per resolved module function (keyed by its parameters' start), the
/// call-site argument bound to each named parameter.
type CallArgs<'a> = HashMap<TextSize, HashMap<&'a str, Vec<&'a Expr>>>;

pub(crate) struct SignatureAnnotations;

impl SignatureAnnotations {
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
        walker.visit_body(&source.ast().body);
        walker.diagnostics
    }
}

/// Collects, per resolved module function, the call-site argument
/// expression bound to each named parameter.
struct LiteralCollector<'a> {
    map: CallArgs<'a>,
    resolved: HashMap<TextSize, &'a Parameters>,
    source: &'a Source,
}

impl<'a> AstVisitor<'a> for LiteralCollector<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr
            && let Some(params) = resolve_call_params(call, &self.resolved)
            && let Some(keywords) = keyword_args(self.source, call, Some(params))
        {
            let bound = self.map.entry(params.range().start()).or_default();
            for arg in keywords.args {
                bound.entry(arg.name).or_default().push(arg.value);
            }
        }
        walk_expr(self, expr);
    }
}

/// The signals a parameter draws toward an inferred annotation, folded
/// into the one scalar type they agree on plus an optional `| None` arm.
#[derive(Default)]
struct SignalSet {
    has_none: bool,
    opaque: bool,
    types: BTreeSet<&'static str>,
}

impl SignalSet {
    /// Folds the signal `expr` contributes, peeling a unary `+`/`-` over
    /// a number so `-1` reads as `int`. A non-literal lands `opaque`.
    fn add(&mut self, expr: &Expr) {
        let inner = match expr {
            Expr::UnaryOp(unary) if matches!(unary.op, UnaryOp::UAdd | UnaryOp::USub) => {
                unary.operand.as_ref()
            }
            _ => expr,
        };
        match inner.as_literal_expr() {
            Some(LiteralExpressionRef::NumberLiteral(number)) => {
                self.types.insert(match &number.value {
                    Number::Int(_) => "int",
                    Number::Float(_) => "float",
                    Number::Complex { .. } => "complex",
                });
            }
            Some(LiteralExpressionRef::StringLiteral(_)) => {
                self.types.insert("str");
            }
            Some(LiteralExpressionRef::BytesLiteral(_)) => {
                self.types.insert("bytes");
            }
            Some(LiteralExpressionRef::BooleanLiteral(_)) => {
                self.types.insert("bool");
            }
            Some(LiteralExpressionRef::NoneLiteral(_)) => self.has_none = true,
            _ => self.opaque = true,
        }
    }

    /// The suggested annotation, or `None` when a non-literal disqualified
    /// the set, the typed signals conflict, or none is typed.
    fn suggestion(&self) -> Option<String> {
        if self.opaque {
            return None;
        }
        let mut types = self.types.iter().copied();
        let only = types.next()?;
        if types.next().is_some() {
            return None;
        }
        Some(if self.has_none {
            format!("{only} | None")
        } else {
            only.to_string()
        })
    }
}

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
        let params_start = params.range().start();
        let receiver = params
            .posonlyargs
            .first()
            .or(params.args.first())
            .map(|p| p.range().start());
        for param in params.iter_non_variadic_params() {
            if param.annotation().is_some() {
                continue;
            }
            if Some(param.range().start()) == receiver
                && matches!(param.name().as_str(), "self" | "cls")
            {
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
        let base = format!("parameter `{name}` has no type annotation");
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

impl<'a> StatementVisitor<'a> for Walker<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::FunctionDef(fd) = stmt {
            self.process_def(fd);
        }
        walk_stmt(self, stmt);
    }
}

/// Builds the [`CallArgs`] index, resolving each in-module callee through
/// `module_call_params` and `keyword_args`.
fn call_argument_literals(source: &Source) -> CallArgs<'_> {
    let mut collector = LiteralCollector {
        map: HashMap::new(),
        resolved: module_call_params(source),
        source,
    };
    collector.visit_body(&source.ast().body);
    collector.map
}

/// True when `fd`'s own body returns a value, a `return` carrying an
/// expression other than a bare `None`. A nested scope's returns and a
/// generator's `yield`s do not count.
fn returns_value(fd: &StmtFunctionDef) -> bool {
    let mut visitor = ReturnStatementVisitor::default();
    visitor.visit_body(&fd.body);
    visitor
        .returns
        .iter()
        .filter_map(|ret| ret.value.as_deref())
        .any(|value| !value.is_none_literal_expr())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_diagnostics::Applicability;

    use super::*;
    use crate::diagnostics::Severity;
    use crate::testing::parse;

    fn first_def(source: &Source) -> &StmtFunctionDef {
        source.ast().body[0]
            .as_function_def_stmt()
            .expect("function def")
    }

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
            signals.add(
                source.ast().body[0]
                    .as_assign_stmt()
                    .expect("assign")
                    .value
                    .as_ref(),
            );
        }
        signals.suggestion()
    }

    #[test]
    fn a_confident_signal_rides_a_display_only_suggestion() {
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

        assert!(report.message.starts_with("parameter"));
    }

    #[test]
    fn a_receiver_and_the_variadics_stay_unreported() {
        let source = parse("class C:\n    def m(self, *args, **kwargs):\n        return args\n");
        let rule = SignatureAnnotations::from_config(&Config::default());
        let diagnostics = rule.lint(&source);
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.starts_with("parameter"))
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
