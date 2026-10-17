//! The semantic-syntax check, reporting the errors Python's `compile`
//! raises past a bare parse. `ruff_python_parser`'s
//! `SemanticSyntaxChecker` supplies the checks, and this module walks the
//! whole tree, each comprehension in the order Python evaluates it,
//! answering each scope query the checker makes from the frames the
//! walk is inside.

use std::cell::OnceCell;

use ruff_python_ast::{
    self as ast, Comprehension, Expr, Parameters, PythonVersion, Stmt,
    statement_visitor::{self, StatementVisitor},
    visitor::source_order::{SourceOrderVisitor, walk_expr, walk_stmt},
};
use ruff_python_parser::semantic_errors::{
    LazyImportContext, SemanticSyntaxChecker, SemanticSyntaxContext, SemanticSyntaxError,
};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use super::Source;
use crate::primitives::{binding::BindingAnalysis, imports::future_annotations_alias};

impl Source {
    /// Returns the first semantic-syntax error this source holds under
    /// `version`, `None` for a source that compiles.
    pub(crate) fn semantic_error(&self, version: PythonVersion) -> Option<SemanticSyntaxError> {
        let body = &self.ast().body;
        let mut walk = Walk {
            bindings: OnceCell::new(),
            checker: SemanticSyntaxChecker::new(),
            error: OnceCell::new(),
            frames: vec![Frame::new(Kind::Module, body)],
            future_annotations: false,
            source: self,
            version,
        };
        walk.visit_body(body);
        walk.error.into_inner()
    }
}

/// One scope the walk is inside, holding the names its body declares
/// `global` and how many loops and `try` statements enclose the statement
/// the walk is visiting within that scope.
struct Frame<'src> {
    globals: Globals<'src>,
    kind: Kind<'src>,
    loops: usize,
    tries: usize,
}

impl<'src> Frame<'src> {
    fn new(kind: Kind<'src>, body: &'src [Stmt]) -> Self {
        let mut globals = Globals::default();
        globals.visit_body(body);
        Self {
            globals,
            kind,
            loops: 0,
            tries: 0,
        }
    }
}

/// The names a scope's body declares `global`, each beside the range of
/// its last declaration, read without entering a nested definition.
#[derive(Default)]
struct Globals<'src>(FxHashMap<&'src str, TextRange>);

impl<'src> StatementVisitor<'src> for Globals<'src> {
    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        match stmt {
            Stmt::Global(global) => {
                self.0.extend(
                    global
                        .names
                        .iter()
                        .map(|name| (name.as_str(), name.range())),
                );
            }
            Stmt::ClassDef(_) | Stmt::FunctionDef(_) => {}
            _ => statement_visitor::walk_stmt(self, stmt),
        }
    }
}

/// The kind of scope a frame opens, where a comprehension's `generator`
/// marks a generator expression rather than a list, set, or dict.
#[derive(Clone, Copy)]
enum Kind<'src> {
    Class,
    Comprehension {
        generator: bool,
        is_async: bool,
    },
    Function {
        is_async: bool,
        parameters: &'src Parameters,
        stmt: &'src Stmt,
    },
    Lambda,
    Module,
}

/// [`SemanticSyntaxContext`] over a source's whole tree, holding the
/// first error reported beside the binding table built at the first
/// `nonlocal` declaration, which reports whether an enclosing function
/// binds the declared name.
struct Walk<'src> {
    bindings: OnceCell<BindingAnalysis>,
    checker: SemanticSyntaxChecker,
    error: OnceCell<SemanticSyntaxError>,
    frames: Vec<Frame<'src>>,
    future_annotations: bool,
    source: &'src Source,
    version: PythonVersion,
}

impl<'src> Walk<'src> {
    /// Runs `check` with the checker beside the walk it reads its
    /// context from.
    fn check(&mut self, check: impl FnOnce(&mut SemanticSyntaxChecker, &Self)) {
        let mut checker = std::mem::take(&mut self.checker);
        check(&mut checker, self);
        self.checker = checker;
    }

    /// Visits a comprehension, evaluating the first iterable in the
    /// enclosing scope and every other clause, then `element`, in the
    /// comprehension's own frame.
    fn comprehension(
        &mut self,
        generators: &'src [Comprehension],
        generator: bool,
        element: impl FnOnce(&mut Self),
    ) {
        let [first, rest @ ..] = generators else {
            unreachable!("invariant: a comprehension always carries one `for` clause");
        };
        self.visit_expr(&first.iter);
        let is_async = generators.iter().any(|clause| clause.is_async);
        self.within(
            Kind::Comprehension {
                generator,
                is_async,
            },
            &[],
            |walk| {
                walk.visit_expr(&first.target);
                for condition in &first.ifs {
                    walk.visit_expr(condition);
                }
                for clause in rest {
                    walk.visit_comprehension(clause);
                }
                element(walk);
            },
        );
    }

    /// Returns the innermost frame.
    fn current(&self) -> &Frame<'src> {
        self.frames
            .last()
            .expect("invariant: the module frame stays open for the whole walk")
    }

    fn current_mut(&mut self) -> &mut Frame<'src> {
        self.frames
            .last_mut()
            .expect("invariant: the module frame stays open for the whole walk")
    }

    /// Yields each frame's kind, innermost first.
    fn kinds(&self) -> impl Iterator<Item = Kind<'src>> {
        self.frames.iter().rev().map(|frame| frame.kind)
    }

    /// Visits `body` as a loop's body, inside the loop the innermost
    /// frame counts.
    fn looping(&mut self, body: &'src [Stmt]) {
        self.current_mut().loops += 1;
        self.visit_body(body);
        self.current_mut().loops -= 1;
    }

    /// Runs `visit` inside a frame of `kind` whose scope declares the
    /// `global` names `body` holds.
    fn within(&mut self, kind: Kind<'src>, body: &'src [Stmt], visit: impl FnOnce(&mut Self)) {
        self.frames.push(Frame::new(kind, body));
        visit(self);
        self.frames.pop();
    }
}

impl SemanticSyntaxContext for Walk<'_> {
    fn future_annotations_or_stub(&self) -> bool {
        self.future_annotations || self.source.source_type.is_stub()
    }

    fn global(&self, name: &str) -> Option<TextRange> {
        self.current().globals.0.get(name).copied()
    }

    fn has_nonlocal_binding(&self, name: &str) -> bool {
        let bindings = self
            .bindings
            .get_or_init(|| BindingAnalysis::new(self.source.ast()));
        self.kinds().skip(1).any(
            |kind| matches!(kind, Kind::Function { stmt, .. } if bindings.scope_binds(stmt, name)),
        )
    }

    fn in_async_context(&self) -> bool {
        self.kinds()
            .find_map(|kind| match kind {
                Kind::Class | Kind::Lambda => Some(false),
                Kind::Function { is_async, .. } => Some(is_async),
                Kind::Comprehension { .. } | Kind::Module => None,
            })
            .unwrap_or(false)
    }

    fn in_await_allowed_context(&self) -> bool {
        self.kinds()
            .find_map(|kind| match kind {
                Kind::Class => Some(false),
                Kind::Comprehension {
                    generator: true, ..
                }
                | Kind::Function { .. }
                | Kind::Lambda => Some(true),
                Kind::Comprehension { .. } | Kind::Module => None,
            })
            .unwrap_or(false)
    }

    fn in_class_body_comprehension(&self) -> bool {
        matches!(
            self.kinds()
                .find(|kind| !matches!(kind, Kind::Comprehension { .. })),
            Some(Kind::Class)
        )
    }

    fn in_function_scope(&self) -> bool {
        matches!(self.current().kind, Kind::Function { .. } | Kind::Lambda)
    }

    fn in_generator_context(&self) -> bool {
        self.kinds().any(|kind| {
            matches!(
                kind,
                Kind::Comprehension {
                    generator: true,
                    ..
                }
            )
        })
    }

    fn in_loop_context(&self) -> bool {
        self.current().loops > 0
    }

    fn in_module_scope(&self) -> bool {
        matches!(self.current().kind, Kind::Module)
    }

    fn in_notebook(&self) -> bool {
        self.source.is_notebook()
    }

    fn in_sync_comprehension(&self) -> bool {
        self.kinds().any(|kind| {
            matches!(
                kind,
                Kind::Comprehension {
                    generator: false,
                    is_async: false
                }
            )
        })
    }

    fn in_yield_allowed_context(&self) -> bool {
        self.kinds()
            .find_map(|kind| match kind {
                Kind::Class | Kind::Comprehension { .. } => Some(false),
                Kind::Function { .. } | Kind::Lambda => Some(true),
                Kind::Module => None,
            })
            .unwrap_or(false)
    }

    fn is_bound_parameter(&self, name: &str) -> bool {
        match self.current().kind {
            Kind::Function { parameters, .. } => parameters.includes(name),
            Kind::Class | Kind::Comprehension { .. } | Kind::Lambda | Kind::Module => false,
        }
    }

    fn lazy_import_context(&self) -> Option<LazyImportContext> {
        match self.current().kind {
            Kind::Function { .. } => Some(LazyImportContext::Function),
            Kind::Class => Some(LazyImportContext::Class),
            Kind::Module if self.current().tries > 0 => Some(LazyImportContext::TryExceptBlocks),
            Kind::Comprehension { .. } | Kind::Lambda | Kind::Module => None,
        }
    }

    fn python_version(&self) -> PythonVersion {
        self.version
    }

    fn report_semantic_error(&self, error: SemanticSyntaxError) {
        let _ = self.error.set(error);
    }

    fn source(&self) -> &str {
        self.source.text()
    }
}

impl<'src> SourceOrderVisitor<'src> for Walk<'src> {
    fn visit_expr(&mut self, expr: &'src Expr) {
        self.check(|checker, walk| checker.visit_expr(expr, walk));
        match expr {
            Expr::DictComp(ast::ExprDictComp {
                key,
                value,
                generators,
                ..
            }) => self.comprehension(generators, false, |walk| {
                if let Some(key) = key {
                    walk.visit_expr(key);
                }
                walk.visit_expr(value);
            }),
            Expr::Generator(ast::ExprGenerator {
                elt, generators, ..
            }) => self.comprehension(generators, true, |walk| walk.visit_expr(elt)),
            Expr::Lambda(lambda) => {
                if let Some(parameters) = &lambda.parameters {
                    self.visit_parameters(parameters);
                }
                self.within(Kind::Lambda, &[], |walk| walk.visit_expr(&lambda.body));
            }
            Expr::ListComp(ast::ExprListComp {
                elt, generators, ..
            })
            | Expr::SetComp(ast::ExprSetComp {
                elt, generators, ..
            }) => self.comprehension(generators, false, |walk| walk.visit_expr(elt)),
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'src Stmt) {
        self.check(|checker, walk| checker.visit_stmt(stmt, walk));
        match stmt {
            Stmt::ClassDef(class) => {
                for decorator in &class.decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = &class.type_params {
                    self.visit_type_params(type_params);
                }
                if let Some(arguments) = &class.arguments {
                    self.visit_arguments(arguments);
                }
                self.within(Kind::Class, &class.body, |walk| {
                    walk.visit_body(&class.body)
                });
            }
            Stmt::For(ast::StmtFor {
                target,
                iter,
                body,
                orelse,
                ..
            }) => {
                self.visit_expr(target);
                self.visit_expr(iter);
                self.looping(body);
                self.visit_body(orelse);
            }
            Stmt::FunctionDef(function) => {
                for decorator in &function.decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = &function.type_params {
                    self.visit_type_params(type_params);
                }
                self.visit_parameters(&function.parameters);
                if let Some(returns) = &function.returns {
                    self.visit_annotation(returns);
                }
                let kind = Kind::Function {
                    is_async: function.is_async,
                    parameters: &function.parameters,
                    stmt,
                };
                self.within(kind, &function.body, |walk| walk.visit_body(&function.body));
            }
            Stmt::ImportFrom(import) => {
                self.future_annotations |=
                    !import.is_lazy && future_annotations_alias(import).is_some();
                walk_stmt(self, stmt);
            }
            Stmt::Try(_) => {
                self.current_mut().tries += 1;
                walk_stmt(self, stmt);
                self.current_mut().tries -= 1;
            }
            Stmt::While(ast::StmtWhile {
                test, body, orelse, ..
            }) => {
                self.visit_expr(test);
                self.looping(body);
                self.visit_body(orelse);
            }
            _ => walk_stmt(self, stmt),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_python_parser::semantic_errors::SemanticSyntaxErrorKind;

    use super::*;
    use crate::testing::{notebook, parse};

    #[rstest]
    #[case::async_comprehension_in_its_first_iterable(
        "async def f():\n    return [z for z in [x async for x in y]]\n"
    )]
    #[case::await_in_a_generator_inside_a_function("def f():\n    return (await x for x in y)\n")]
    #[case::await_in_a_lambda_default("async def f():\n    return lambda a=await g(): a\n")]
    #[case::break_in_a_loop_finally(
        "for i in []:\n    try:\n        pass\n    finally:\n        break\n"
    )]
    #[case::docstring_ahead_of_a_future_import(
        "\"\"\"Doc.\"\"\"\nfrom __future__ import annotations\nimport os\n"
    )]
    #[case::empty("")]
    #[case::future_import_after_a_comment("# lead\nfrom __future__ import annotations\n")]
    #[case::future_imports_naming_two_features("from __future__ import annotations, division\n")]
    #[case::future_import_leading("from __future__ import annotations\nimport os\n")]
    #[case::lazy_import_in_a_match_arm("match 1:\n    case 1:\n        lazy import os\n")]
    #[case::lazy_import_in_a_with_block("import c\nwith c.nullcontext():\n    lazy import os\n")]
    #[case::lazy_imports_at_module_scope("lazy import os\nlazy from os import path\n")]
    #[case::nonlocal_across_a_class(
        "def outer():\n    x = 1\n    class C:\n        def m(self):\n            nonlocal x\n"
    )]
    #[case::nonlocal_bound_later_in_the_enclosing_function(
        "def outer():\n    def inner():\n        nonlocal x\n    x = 1\n"
    )]
    #[case::nonlocal_in_a_class_body("def outer():\n    x = 1\n    class C:\n        nonlocal x\n")]
    #[case::nonlocal_naming_an_enclosing_local(
        "def outer():\n    x = 1\n    def inner():\n        nonlocal x\n        x = 2\n"
    )]
    #[case::nonlocal_naming_an_enclosing_parameter(
        "def outer(x):\n    def inner():\n        nonlocal x\n"
    )]
    #[case::nonlocal_naming_an_enclosing_walrus(
        "def outer():\n    (x := 1)\n    def inner():\n        nonlocal x\n"
    )]
    #[case::ordinary_imports("import os\nfrom collections import deque\n")]
    #[case::unary_plus_pattern("match 1:\n    case +1:\n        pass\n")]
    fn semantic_error_clears_a_compiling_module(#[case] src: &str) {
        assert_eq!(parse(src).semantic_error(PythonVersion::PY315), None);
    }

    #[test]
    fn semantic_error_clears_a_top_level_await_in_a_notebook() {
        let source = notebook(&["import asyncio\n", "await asyncio.sleep(0)\n"]);
        assert_eq!(source.semantic_error(PythonVersion::PY315), None);
    }

    #[rstest]
    #[case("import os\n\nfrom __future__ import annotations\n")]
    #[case("x = 1\nfrom __future__ import division\n")]
    #[case("\"\"\"Doc.\"\"\"\nimport os\nfrom __future__ import annotations\n")]
    fn semantic_error_flags_a_demoted_future_import(#[case] src: &str) {
        let error = parse(src)
            .semantic_error(PythonVersion::PY315)
            .expect("late future import reports");
        assert_eq!(error.kind, SemanticSyntaxErrorKind::LateFutureImport);
    }

    #[test]
    fn semantic_error_holds_only_the_first_report() {
        let source = parse(
            "import os\nfrom __future__ import annotations\nfrom __future__ import division\n",
        );
        let error = source
            .semantic_error(PythonVersion::PY315)
            .expect("first late future import reports");
        assert_eq!(
            source.slice(error.range),
            "from __future__ import annotations"
        );
    }

    #[rstest]
    #[case::annotated_global(
        "def f():\n    global x\n    x: int = 1\n",
        SemanticSyntaxErrorKind::AnnotatedGlobal("x".into())
    )]
    #[case::break_in_a_loop_else(
        "for i in []:\n    pass\nelse:\n    break\n",
        SemanticSyntaxErrorKind::BreakOutsideLoop
    )]
    #[case::break_outside_a_loop("break\n", SemanticSyntaxErrorKind::BreakOutsideLoop)]
    #[case::continue_in_a_class_inside_a_loop(
        "for i in []:\n    class C:\n        continue\n",
        SemanticSyntaxErrorKind::ContinueOutsideLoop
    )]
    #[case::duplicate_match_key(
        "match x:\n    case {\"a\": 1, \"a\": 2}:\n        pass\n",
        SemanticSyntaxErrorKind::DuplicateMatchKey("\"a\"".into())
    )]
    #[case::global_parameter(
        "def f(x):\n    global x\n",
        SemanticSyntaxErrorKind::GlobalParameter("x".into())
    )]
    #[case::lazy_from_future_import(
        "lazy from __future__ import annotations\n",
        SemanticSyntaxErrorKind::LazyFutureImport
    )]
    #[case::lazy_star_import("lazy from os import *\n", SemanticSyntaxErrorKind::LazyImportStar)]
    #[case::nonlocal_at_module_scope(
        "nonlocal x\n",
        SemanticSyntaxErrorKind::NonlocalDeclarationAtModuleLevel
    )]
    #[case::nonlocal_with_no_enclosing_function(
        "def f():\n    nonlocal x\n",
        SemanticSyntaxErrorKind::NonlocalWithoutBinding("x".into())
    )]
    #[case::return_in_a_class_body(
        "class C:\n    return 1\n",
        SemanticSyntaxErrorKind::ReturnOutsideFunction
    )]
    #[case::return_with_a_value_in_an_async_generator(
        "async def f():\n    yield 1\n    return 2\n",
        SemanticSyntaxErrorKind::ReturnInGenerator
    )]
    #[case::star_import_in_a_function(
        "def f():\n    from os import *\n",
        SemanticSyntaxErrorKind::NonModuleImportStar("os".into())
    )]
    #[case::walrus_in_a_class_body_comprehension(
        "class C:\n    [y := 1 for x in z]\n",
        SemanticSyntaxErrorKind::NamedExpressionInClassBodyComprehension
    )]
    #[case::yield_from_in_an_async_function(
        "async def f():\n    yield from x\n",
        SemanticSyntaxErrorKind::YieldFromInAsyncFunction
    )]
    fn semantic_error_names_the_kind_python_reports(
        #[case] src: &str,
        #[case] kind: SemanticSyntaxErrorKind,
    ) {
        let error = parse(src)
            .semantic_error(PythonVersion::PY315)
            .expect("the source does not compile");
        assert_eq!(error.kind, kind);
    }

    #[test]
    fn semantic_error_reads_a_notebook_as_one_module() {
        let source = notebook(&["import os\n", "from __future__ import annotations\n"]);
        let error = source
            .semantic_error(PythonVersion::PY315)
            .expect("late future import reports");
        assert_eq!(error.kind, SemanticSyntaxErrorKind::LateFutureImport);
    }

    #[rstest]
    #[case::parameter_annotation_under_the_future_import(
        "from __future__ import annotations\ndef f(x: (y := 1)): pass\n",
        true
    )]
    #[case::parameter_annotation_without_the_future_import("def f(x: (y := 1)): pass\n", false)]
    fn semantic_error_reads_the_future_annotations_import(
        #[case] src: &str,
        #[case] rejects: bool,
    ) {
        assert_eq!(
            parse(src).semantic_error(PythonVersion::PY313).is_some(),
            rejects
        );
    }

    #[rstest]
    #[case::await_at_module_scope("await x\n")]
    #[case::await_in_a_class_inside_an_async_function(
        "async def f():\n    class C:\n        await x\n"
    )]
    #[case::await_in_a_lambda("lambda: await x\n")]
    #[case::await_in_a_sync_function("def f():\n    await x\n")]
    #[case::global_after_an_assignment("def f():\n    x = 1\n    global x\n")]
    #[case::global_after_a_read("def f():\n    print(x)\n    global x\n")]
    #[case::lazy_import_in_a_class("class C:\n    lazy import os\n")]
    #[case::lazy_import_in_a_finally("try:\n    pass\nfinally:\n    lazy import os\n")]
    #[case::lazy_import_in_a_function("def f():\n    lazy import json\n")]
    #[case::lazy_import_in_a_function_inside_a_try(
        "try:\n    def f():\n        lazy import os\nexcept Exception:\n    pass\n"
    )]
    #[case::lazy_import_in_a_try("try:\n    lazy import os\nexcept ImportError:\n    pass\n")]
    #[case::lazy_import_in_an_except("try:\n    pass\nexcept ImportError:\n    lazy import os\n")]
    #[case::lazy_import_in_a_try_else(
        "try:\n    pass\nexcept ImportError:\n    pass\nelse:\n    lazy import os\n"
    )]
    #[case::nonlocal_naming_a_class_attribute(
        "class C:\n    x = 1\n    def m(self):\n        nonlocal x\n"
    )]
    #[case::nonlocal_naming_a_comprehension_target(
        "def outer():\n    [x for x in y]\n    def inner():\n        nonlocal x\n"
    )]
    #[case::nonlocal_naming_a_global_write(
        "def outer():\n    global x\n    x = 1\n    def inner():\n        nonlocal x\n"
    )]
    #[case::nonlocal_naming_a_module_binding("x = 1\ndef f():\n    nonlocal x\n")]
    #[case::nonlocal_with_no_enclosing_binding(
        "def outer():\n    def inner():\n        nonlocal x\n"
    )]
    #[case::walrus_in_an_annotation("x: (y := 1) = 2\n")]
    #[case::yield_at_module_scope("yield 1\n")]
    #[case::yield_in_a_class_inside_a_function("def f():\n    class C:\n        yield 1\n")]
    #[case::yield_in_a_comprehension("def f():\n    return [(yield) for x in y]\n")]
    fn semantic_error_rejects_what_python_rejects(#[case] src: &str) {
        assert!(parse(src).semantic_error(PythonVersion::PY315).is_some());
    }
}
