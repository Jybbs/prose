//! Per-`Source` binding-resolution table.
//!
//! Walks the module once and records, for every name introduced or
//! shadowed in a lexical scope, the offsets of every write and read.
//! Consuming rules query the table by `BindingId` or by `&Stmt`
//! rather than driving their own walk.
//!
//! ## Scope model
//!
//! Each scope is one of `Module`, `Function`, `Class`, or
//! `Comprehension`. The scope stack mirrors source-order nesting:
//! every `function-def`, `lambda`, `class-def`, and comprehension
//! pushes a frame, every comprehension's first generator iterable
//! evaluates in the enclosing scope, every walrus target lifts to
//! the nearest non-comprehension scope, and class-scope names are
//! invisible to nested functions and comprehensions.

use std::collections::{BTreeMap, HashMap};

use ruff_python_ast::visitor::{walk_arguments, walk_expr, walk_parameters, walk_stmt, Visitor};
use ruff_python_ast::{
    Expr, ExprDictComp, ExprGenerator, ExprLambda, ExprList, ExprListComp, ExprNamed, ExprSetComp,
    ExprTuple, Identifier, ModModule, Parameters, Stmt, StmtAnnAssign, StmtAssign, StmtAugAssign,
    StmtClassDef, StmtFor, StmtFunctionDef, StmtImport, StmtImportFrom, StmtTry, StmtWith,
};
use ruff_text_size::{Ranged, TextSize};
use serde::Serialize;

/// Stable handle to a binding in `BindingAnalysis`. Cheap to copy.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct BindingId(u32);

/// Stable handle to a scope in `BindingAnalysis`. Cheap to copy.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct ScopeId(u32);

/// Categories of write event recorded against a binding.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) enum BindingKind {
    Assignment,
    AugAssign,
    ClassDef,
    Comprehension,
    ExceptHandler,
    For,
    FunctionDef,
    Import,
    Parameter,
    Walrus,
    With,
}

/// Categories of lexical scope.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) enum ScopeKind {
    Class,
    Comprehension,
    Function,
    Module,
}

/// One named binding in some scope, with every observed write and read.
#[derive(Debug, Serialize)]
pub(crate) struct Binding {
    kinds: Vec<BindingKind>,
    name: String,
    read_offsets: Vec<TextSize>,
    scope: ScopeId,
    write_offsets: Vec<TextSize>,
}

/// One lexical scope plus its binding table keyed by name.
#[derive(Debug, Serialize)]
pub(crate) struct Scope {
    bindings: BTreeMap<String, BindingId>,
    kind: ScopeKind,
    parent: Option<ScopeId>,
}

/// Module-wide binding-resolution table.
#[derive(Debug, Serialize)]
pub struct BindingAnalysis {
    bindings: Vec<Binding>,
    #[serde(skip)]
    function_scope_at: HashMap<TextSize, ScopeId>,
    scopes: Vec<Scope>,
}

impl BindingAnalysis {
    /// Walks `module` once and returns the resulting binding table.
    pub(crate) fn new(module: &ModModule) -> Self {
        let mut builder = Builder::new();
        builder.visit_body(&module.body);
        builder.finish()
    }

    fn binding(&self, id: BindingId) -> &Binding {
        &self.bindings[id.0 as usize]
    }

    /// Returns the number of write events recorded for `binding`.
    pub(crate) fn assignment_count(&self, binding: BindingId) -> usize {
        self.binding(binding).write_offsets.len()
    }

    /// Returns the recorded write kinds for `binding`, in insertion
    /// order and without duplicates.
    pub(crate) fn binding_kinds(&self, binding: BindingId) -> &[BindingKind] {
        &self.binding(binding).kinds
    }

    /// Returns the source-text name of `binding`.
    pub(crate) fn binding_name(&self, binding: BindingId) -> &str {
        &self.binding(binding).name
    }

    /// Returns the binding ids declared directly inside the local
    /// scope of `stmt`. `stmt` must be a `Stmt::FunctionDef`; any
    /// other statement yields an empty iterator.
    pub(crate) fn bindings_in_scope(&self, stmt: &Stmt) -> impl Iterator<Item = BindingId> + '_ {
        self.function_scope_at
            .get(&stmt.range().start())
            .copied()
            .into_iter()
            .flat_map(move |s| self.scopes[s.0 as usize].bindings.values().copied())
    }

    /// Returns the offset of the earliest recorded write of `binding`.
    pub(crate) fn first_write_offset(&self, binding: BindingId) -> TextSize {
        self.binding(binding).write_offsets[0]
    }

    /// Returns `true` when `name` has a module-scope write event at
    /// any offset strictly less than `offset`.
    pub(crate) fn is_defined_before(&self, name: &str, offset: TextSize) -> bool {
        self.scopes[0]
            .bindings
            .get(name)
            .and_then(|&id| self.binding(id).write_offsets.first())
            .is_some_and(|&first| first < offset)
    }

    /// Returns the number of read events recorded for `binding`.
    pub(crate) fn usage_count(&self, binding: BindingId) -> usize {
        self.binding(binding).read_offsets.len()
    }
}

struct Builder {
    bindings: Vec<Binding>,
    function_scope_at: HashMap<TextSize, ScopeId>,
    scope_stack: Vec<ScopeId>,
    scopes: Vec<Scope>,
}

impl Builder {
    fn new() -> Self {
        let mut builder = Self {
            bindings: Vec::new(),
            function_scope_at: HashMap::new(),
            scope_stack: Vec::new(),
            scopes: Vec::new(),
        };
        builder.push_scope(ScopeKind::Module, None);
        builder
    }

    fn current_scope(&self) -> ScopeId {
        *self
            .scope_stack
            .last()
            .expect("invariant: module scope is always present")
    }

    fn enter_class(&mut self, class: &StmtClassDef) {
        for decorator in &class.decorator_list {
            self.visit_expr(&decorator.expression);
        }
        if let Some(arguments) = &class.arguments {
            walk_arguments(self, arguments);
        }
        self.record_identifier(&class.name, BindingKind::ClassDef);
        let parent = Some(self.current_scope());
        self.push_scope(ScopeKind::Class, parent);
        self.visit_body(&class.body);
        self.pop_scope();
    }

    fn enter_comprehension(
        &mut self,
        generators: &[ruff_python_ast::Comprehension],
        elements: &[&Expr],
    ) {
        let Some((first, rest)) = generators.split_first() else {
            unreachable!(
                "invariant: comprehension carries at least one generator (parser guarantee)"
            );
        };
        self.visit_expr(&first.iter);
        let parent = Some(self.current_scope());
        self.push_scope(ScopeKind::Comprehension, parent);
        self.record_target(&first.target, BindingKind::Comprehension);
        for guard in &first.ifs {
            self.visit_expr(guard);
        }
        for generator in rest {
            self.visit_expr(&generator.iter);
            self.record_target(&generator.target, BindingKind::Comprehension);
            for guard in &generator.ifs {
                self.visit_expr(guard);
            }
        }
        for element in elements {
            self.visit_expr(element);
        }
        self.pop_scope();
    }

    fn enter_function(&mut self, function: &StmtFunctionDef, stmt_start: TextSize) {
        for decorator in &function.decorator_list {
            self.visit_expr(&decorator.expression);
        }
        walk_parameters(self, &function.parameters);
        if let Some(returns) = &function.returns {
            self.visit_expr(returns);
        }
        self.record_identifier(&function.name, BindingKind::FunctionDef);
        let parent = Some(self.current_scope());
        let function_scope = self.push_scope(ScopeKind::Function, parent);
        self.function_scope_at.insert(stmt_start, function_scope);
        self.record_parameters(&function.parameters);
        self.visit_body(&function.body);
        self.pop_scope();
    }

    fn enter_lambda(&mut self, lambda: &ExprLambda) {
        if let Some(parameters) = &lambda.parameters {
            walk_parameters(self, parameters);
        }
        let parent = Some(self.current_scope());
        self.push_scope(ScopeKind::Function, parent);
        if let Some(parameters) = &lambda.parameters {
            self.record_parameters(parameters);
        }
        self.visit_expr(&lambda.body);
        self.pop_scope();
    }

    fn finish(self) -> BindingAnalysis {
        BindingAnalysis {
            scopes: self.scopes,
            bindings: self.bindings,
            function_scope_at: self.function_scope_at,
        }
    }

    fn pop_scope(&mut self) {
        self.scope_stack
            .pop()
            .expect("invariant: pop balanced with push");
    }

    fn push_scope(&mut self, kind: ScopeKind, parent: Option<ScopeId>) -> ScopeId {
        let id = ScopeId(u32::try_from(self.scopes.len()).expect("scope count fits in u32"));
        self.scopes.push(Scope {
            kind,
            parent,
            bindings: BTreeMap::new(),
        });
        self.scope_stack.push(id);
        id
    }

    fn record_identifier(&mut self, identifier: &Identifier, kind: BindingKind) {
        self.record_write(identifier.as_str(), identifier.range().start(), kind);
    }

    fn record_parameters(&mut self, parameters: &Parameters) {
        for parameter in parameters.iter_source_order() {
            self.record_identifier(parameter.name(), BindingKind::Parameter);
        }
    }

    fn record_read(&mut self, name: &str, offset: TextSize) {
        let innermost = self.current_scope();
        for &scope_id in self.scope_stack.iter().rev() {
            let scope = &self.scopes[scope_id.0 as usize];
            if scope_id != innermost && matches!(scope.kind, ScopeKind::Class) {
                continue;
            }
            if let Some(&binding_id) = scope.bindings.get(name) {
                self.bindings[binding_id.0 as usize]
                    .read_offsets
                    .push(offset);
                return;
            }
        }
    }

    fn record_target(&mut self, target: &Expr, kind: BindingKind) {
        match target {
            Expr::Name(name) => self.record_write(name.id.as_str(), name.range().start(), kind),
            Expr::Tuple(ExprTuple { elts, .. }) | Expr::List(ExprList { elts, .. }) => {
                for element in elts {
                    self.record_target(element, kind);
                }
            }
            Expr::Starred(starred) => self.record_target(&starred.value, kind),
            _ => walk_expr(self, target),
        }
    }

    fn record_walrus(&mut self, named: &ExprNamed) {
        self.visit_expr(&named.value);
        let Some(name) = named.target.as_name_expr() else {
            unreachable!("invariant: walrus target is always Expr::Name (parser guarantee)");
        };
        let scope = self
            .scope_stack
            .iter()
            .rev()
            .copied()
            .find(|&id| !matches!(self.scopes[id.0 as usize].kind, ScopeKind::Comprehension))
            .expect("invariant: module scope is always present");
        self.record_write_in(
            scope,
            name.id.as_str(),
            name.range().start(),
            BindingKind::Walrus,
        );
    }

    fn record_write(&mut self, name: &str, offset: TextSize, kind: BindingKind) {
        let scope = self.current_scope();
        self.record_write_in(scope, name, offset, kind);
    }

    fn record_write_in(&mut self, scope: ScopeId, name: &str, offset: TextSize, kind: BindingKind) {
        let scope_data = &mut self.scopes[scope.0 as usize];
        let binding_id = if let Some(&id) = scope_data.bindings.get(name) {
            id
        } else {
            let id =
                BindingId(u32::try_from(self.bindings.len()).expect("binding count fits in u32"));
            scope_data.bindings.insert(name.to_owned(), id);
            self.bindings.push(Binding {
                name: name.to_owned(),
                scope,
                kinds: Vec::new(),
                write_offsets: Vec::new(),
                read_offsets: Vec::new(),
            });
            id
        };
        let binding = &mut self.bindings[binding_id.0 as usize];
        if !binding.kinds.contains(&kind) {
            binding.kinds.push(kind);
        }
        binding.write_offsets.push(offset);
    }

    fn visit_ann_assign(&mut self, node: &StmtAnnAssign) {
        self.visit_expr(&node.annotation);
        if let Some(value) = &node.value {
            self.visit_expr(value);
        }
        if node.target.is_name_expr() {
            self.record_target(&node.target, BindingKind::Assignment);
        }
    }

    fn visit_assign(&mut self, node: &StmtAssign) {
        self.visit_expr(&node.value);
        for target in &node.targets {
            self.record_target(target, BindingKind::Assignment);
        }
    }

    fn visit_aug_assign(&mut self, node: &StmtAugAssign) {
        if let Some(name) = node.target.as_name_expr() {
            self.record_read(name.id.as_str(), name.range().start());
            self.visit_expr(&node.value);
            self.record_write(
                name.id.as_str(),
                name.range().start(),
                BindingKind::AugAssign,
            );
        } else {
            self.visit_expr(&node.value);
            walk_expr(self, &node.target);
        }
    }

    fn visit_for(&mut self, node: &StmtFor) {
        self.visit_expr(&node.iter);
        self.record_target(&node.target, BindingKind::For);
        self.visit_body(&node.body);
        self.visit_body(&node.orelse);
    }

    fn visit_import(&mut self, node: &StmtImport) {
        for alias in &node.names {
            let bound = alias
                .asname
                .as_ref()
                .map_or_else(|| top_level_module(alias.name.as_str()), |id| id.as_str());
            self.record_write(bound, alias.range().start(), BindingKind::Import);
        }
    }

    fn visit_import_from(&mut self, node: &StmtImportFrom) {
        for alias in &node.names {
            let bound = alias.asname.as_ref().unwrap_or(&alias.name);
            self.record_write(bound.as_str(), alias.range().start(), BindingKind::Import);
        }
    }

    fn visit_try(&mut self, node: &StmtTry) {
        self.visit_body(&node.body);
        for handler in &node.handlers {
            let ruff_python_ast::ExceptHandler::ExceptHandler(eh) = handler;
            if let Some(type_) = &eh.type_ {
                self.visit_expr(type_);
            }
            if let Some(name) = &eh.name {
                self.record_identifier(name, BindingKind::ExceptHandler);
            }
            self.visit_body(&eh.body);
        }
        self.visit_body(&node.orelse);
        self.visit_body(&node.finalbody);
    }

    fn visit_with(&mut self, node: &StmtWith) {
        for item in &node.items {
            self.visit_expr(&item.context_expr);
            if let Some(target) = &item.optional_vars {
                self.record_target(target, BindingKind::With);
            }
        }
        self.visit_body(&node.body);
    }
}

impl<'a> Visitor<'a> for Builder {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                if name.ctx.is_load() {
                    self.record_read(name.id.as_str(), name.range().start());
                }
            }
            Expr::Named(named) => self.record_walrus(named),
            Expr::Lambda(lambda) => self.enter_lambda(lambda),
            Expr::ListComp(ExprListComp {
                generators, elt, ..
            })
            | Expr::SetComp(ExprSetComp {
                generators, elt, ..
            })
            | Expr::Generator(ExprGenerator {
                generators, elt, ..
            }) => self.enter_comprehension(generators, &[elt]),
            Expr::DictComp(ExprDictComp {
                generators,
                key,
                value,
                ..
            }) => self.enter_comprehension(generators, &[key, value]),
            _ => walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::AnnAssign(node) => self.visit_ann_assign(node),
            Stmt::Assign(node) => self.visit_assign(node),
            Stmt::AugAssign(node) => self.visit_aug_assign(node),
            Stmt::ClassDef(node) => self.enter_class(node),
            Stmt::For(node) => self.visit_for(node),
            Stmt::FunctionDef(node) => self.enter_function(node, stmt.range().start()),
            Stmt::Global(_) | Stmt::Nonlocal(_) => {}
            Stmt::Import(node) => self.visit_import(node),
            Stmt::ImportFrom(node) => self.visit_import_from(node),
            Stmt::Try(node) => self.visit_try(node),
            Stmt::With(node) => self.visit_with(node),
            _ => walk_stmt(self, stmt),
        }
    }
}

/// Returns the segment of `dotted` before the first `.`. Matches
/// Python's `import a.b.c` shape, which binds `a` rather than the
/// full dotted path.
pub(crate) fn top_level_module(dotted: &str) -> &str {
    dotted.split_once('.').map_or(dotted, |(head, _)| head)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::test_support::parse;

    fn analyze(src: &str) -> BindingAnalysis {
        BindingAnalysis::new(parse(src).ast())
    }

    fn module_binding(analysis: &BindingAnalysis, name: &str) -> BindingId {
        analysis.scopes[0]
            .bindings
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("no module-scope binding for {name:?}"))
    }

    #[test]
    fn bindings_in_scope_iterates_function_locals() {
        let source = parse("def f(a, b):\n    c = a + b\n    return c\n");
        let analysis = BindingAnalysis::new(source.ast());
        let stmt = &source.ast().body[0];
        let names: Vec<&str> = analysis
            .bindings_in_scope(stmt)
            .map(|id| analysis.bindings[id.0 as usize].name.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn bindings_in_scope_returns_empty_for_non_function_stmt() {
        let source = parse("x = 1\n");
        let analysis = BindingAnalysis::new(source.ast());
        let stmt = &source.ast().body[0];
        assert!(analysis.bindings_in_scope(stmt).next().is_none());
    }

    #[test]
    fn is_defined_before_returns_false_for_undefined_name() {
        let analysis = analyze("x = 1\n");
        assert!(!analysis.is_defined_before("y", TextSize::new(100)));
    }

    #[test]
    fn is_defined_before_returns_false_when_only_write_is_after_offset() {
        let analysis = analyze("x = 1\n");
        assert!(!analysis.is_defined_before("x", TextSize::new(0)));
    }

    #[test]
    fn is_defined_before_returns_true_for_prior_module_write() {
        let analysis = analyze("x = 1\nprint(x)\n");
        assert!(analysis.is_defined_before("x", TextSize::new(10)));
    }

    #[test]
    fn top_level_module_returns_first_segment() {
        assert_eq!(top_level_module("a"), "a");
        assert_eq!(top_level_module("a.b"), "a");
        assert_eq!(top_level_module("a.b.c"), "a");
        assert_eq!(top_level_module(""), "");
    }

    proptest! {
        #[test]
        fn closure_binding_is_independent_of_outer_same_name(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!(
                "{name} = 1\ndef inner():\n    {name} = 2\n    return {name}\n",
            );
            let analysis = analyze(&program);
            let outer = module_binding(&analysis, &name);
            let inner_scope = analysis
                .scopes
                .iter()
                .find(|s| matches!(s.kind, ScopeKind::Function))
                .expect("inner is a function scope");
            let inner = *inner_scope
                .bindings
                .get(&name)
                .expect("inner shadows name");
            prop_assert_ne!(outer, inner);
            prop_assert_eq!(analysis.usage_count(outer), 0);
            prop_assert_eq!(analysis.usage_count(inner), 1);
        }

        #[test]
        fn single_use_name_reports_usage_count_one(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!("{name} = 1\nprint({name})\n");
            let analysis = analyze(&program);
            let id = module_binding(&analysis, &name);
            prop_assert_eq!(analysis.usage_count(id), 1);
        }

        #[test]
        fn unread_name_reports_usage_count_zero(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!("{name} = 1\n");
            let analysis = analyze(&program);
            let id = module_binding(&analysis, &name);
            prop_assert_eq!(analysis.usage_count(id), 0);
        }
    }
}
