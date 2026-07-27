//! Module-scope semantic-syntax check, the ground Python's `compile`
//! covers past a bare parse. `ruff_python_parser`'s
//! `SemanticSyntaxChecker` supplies the checks and this module supplies
//! the context it reads, answering as the module scope every visited
//! statement sits in.

use std::cell::OnceCell;

use ruff_python_ast::PythonVersion;
use ruff_python_parser::semantic_errors::{
    LazyImportContext, SemanticSyntaxChecker, SemanticSyntaxContext, SemanticSyntaxError,
};
use ruff_text_size::TextRange;

use crate::source::Source;

/// [`SemanticSyntaxContext`] over a module's own statements, holding the
/// first error reported. The scope-dependent answers cover module scope
/// alone and stay permissive elsewhere.
struct ModuleScope<'src> {
    error: OnceCell<SemanticSyntaxError>,
    module: &'src Source,
    version: PythonVersion,
}

impl SemanticSyntaxContext for ModuleScope<'_> {
    fn future_annotations_or_stub(&self) -> bool {
        false
    }

    fn global(&self, _name: &str) -> Option<TextRange> {
        None
    }

    fn has_nonlocal_binding(&self, _name: &str) -> bool {
        true
    }

    fn in_async_context(&self) -> bool {
        false
    }

    fn in_await_allowed_context(&self) -> bool {
        true
    }

    fn in_class_body_comprehension(&self) -> bool {
        false
    }

    fn in_function_scope(&self) -> bool {
        false
    }

    fn in_generator_context(&self) -> bool {
        true
    }

    fn in_loop_context(&self) -> bool {
        true
    }

    fn in_module_scope(&self) -> bool {
        true
    }

    fn in_notebook(&self) -> bool {
        self.module.is_notebook()
    }

    fn in_sync_comprehension(&self) -> bool {
        false
    }

    fn in_yield_allowed_context(&self) -> bool {
        true
    }

    fn is_bound_parameter(&self, _name: &str) -> bool {
        false
    }

    fn lazy_import_context(&self) -> Option<LazyImportContext> {
        None
    }

    fn python_version(&self) -> PythonVersion {
        self.version
    }

    fn report_semantic_error(&self, error: SemanticSyntaxError) {
        let _ = self.error.set(error);
    }

    fn source(&self) -> &str {
        self.module.text()
    }
}

pub(super) fn compile_gate(source: &Source, version: PythonVersion) -> Option<PythonVersion> {
    first_semantic_error(source, version)
        .is_none()
        .then_some(version)
}

/// Returns the first semantic-syntax error across `source`'s module-level
/// statements, `None` for a module that compiles. The walk stops at
/// module scope and carries the checker's `__future__` boundary state
/// across the run.
pub(super) fn first_semantic_error(
    source: &Source,
    version: PythonVersion,
) -> Option<SemanticSyntaxError> {
    let context = ModuleScope {
        error: OnceCell::new(),
        module: source,
        version,
    };
    let mut checker = SemanticSyntaxChecker::new();
    for stmt in &source.ast().body {
        checker.visit_stmt(stmt, &context);
    }
    context.error.into_inner()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use ruff_python_parser::semantic_errors::SemanticSyntaxErrorKind;

    use super::*;
    use crate::testing::{notebook, parse};

    #[rstest]
    #[case("from __future__ import annotations\nimport os\n")]
    #[case("\"\"\"Doc.\"\"\"\nfrom __future__ import annotations\nimport os\n")]
    #[case("# lead\nfrom __future__ import annotations\n")]
    #[case("from __future__ import annotations, division\n")]
    #[case("import os\nfrom collections import deque\n")]
    #[case("")]
    fn first_semantic_error_clears_a_compiling_module(#[case] src: &str) {
        assert!(first_semantic_error(&parse(src), PythonVersion::default()).is_none());
    }

    #[rstest]
    #[case("import os\n\nfrom __future__ import annotations\n")]
    #[case("x = 1\nfrom __future__ import division\n")]
    #[case("\"\"\"Doc.\"\"\"\nimport os\nfrom __future__ import annotations\n")]
    fn first_semantic_error_flags_a_demoted_future_import(#[case] src: &str) {
        let error = first_semantic_error(&parse(src), PythonVersion::default())
            .expect("late future import reports");
        assert_eq!(error.kind, SemanticSyntaxErrorKind::LateFutureImport);
    }

    #[test]
    fn first_semantic_error_holds_only_the_first_report() {
        let source = parse(
            "import os\nfrom __future__ import annotations\nfrom __future__ import division\n",
        );
        let error = first_semantic_error(&source, PythonVersion::default())
            .expect("first late future import reports");
        assert_eq!(
            source.slice(error.range),
            "from __future__ import annotations"
        );
    }

    #[test]
    fn first_semantic_error_reads_a_notebook_as_one_module() {
        let source = notebook(&["import os\n", "from __future__ import annotations\n"]);
        let error = first_semantic_error(&source, PythonVersion::default())
            .expect("late future import reports");
        assert_eq!(error.kind, SemanticSyntaxErrorKind::LateFutureImport);
    }

    #[test]
    fn module_scope_derives_its_answers_from_the_wrapped_module() {
        let source = parse("x = 1\n");
        let context = ModuleScope {
            error: OnceCell::new(),
            module: &source,
            version: PythonVersion::PY313,
        };
        assert_eq!(context.source(), "x = 1\n");
        assert_eq!(context.python_version(), PythonVersion::PY313);
        assert!(context.in_module_scope());
        assert!(!context.in_function_scope());
        assert!(!context.in_notebook());

        let cells = notebook(&["x = 1\n"]);
        let cell_context = ModuleScope {
            error: OnceCell::new(),
            module: &cells,
            version: PythonVersion::default(),
        };
        assert!(cell_context.in_notebook());
    }

    #[test]
    fn module_scope_holds_constants_for_the_scopes_it_does_not_model() {
        let source = parse("x = 1\n");
        let context = ModuleScope {
            error: OnceCell::new(),
            module: &source,
            version: PythonVersion::default(),
        };
        assert!(context.global("x").is_none());
        assert!(context.lazy_import_context().is_none());
        assert!(context.has_nonlocal_binding("x"));
        assert!(context.in_await_allowed_context());
        assert!(context.in_generator_context());
        assert!(context.in_loop_context());
        assert!(context.in_yield_allowed_context());
        assert!(!context.future_annotations_or_stub());
        assert!(!context.in_async_context());
        assert!(!context.in_class_body_comprehension());
        assert!(!context.in_sync_comprehension());
        assert!(!context.is_bound_parameter("x"));
    }
}
