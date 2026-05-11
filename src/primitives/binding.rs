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

use ruff_python_ast::visitor::{walk_expr, walk_stmt, Visitor};
use ruff_python_ast::{
    Expr, ExprContext, ExprDictComp, ExprGenerator, ExprLambda, ExprListComp, ExprNamed,
    ExprSetComp, Identifier, ModModule, Parameters, Stmt, StmtAnnAssign, StmtAssign, StmtAugAssign,
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

const MODULE_SCOPE: ScopeId = ScopeId(0);

#[allow(dead_code, reason = "consumer rules #62 and #70 land later in 0.2.0")]
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

    /// Returns `true` when `name` has a module-scope write event at
    /// any offset strictly less than `offset`.
    pub(crate) fn is_defined_before(&self, name: &str, offset: TextSize) -> bool {
        self.scopes[MODULE_SCOPE.0 as usize]
            .bindings
            .get(name)
            .is_some_and(|&id| self.binding(id).write_offsets.iter().any(|&o| o < offset))
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
            self.visit_arguments(arguments);
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
        let (first, rest) = generators
            .split_first()
            .expect("invariant: comprehension carries at least one generator");
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
        self.walk_parameter_defaults(&function.parameters);
        for parameter in function.parameters.iter_source_order() {
            if let Some(annotation) = parameter.annotation() {
                self.visit_expr(annotation);
            }
        }
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
            self.walk_parameter_defaults(parameters);
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
            Expr::Name(name) => {
                self.record_write(name.id.as_str(), name.range().start(), kind);
            }
            Expr::Tuple(tuple) => {
                for element in &tuple.elts {
                    self.record_target(element, kind);
                }
            }
            Expr::List(list) => {
                for element in &list.elts {
                    self.record_target(element, kind);
                }
            }
            Expr::Starred(starred) => self.record_target(&starred.value, kind),
            Expr::Attribute(attribute) => self.visit_expr(&attribute.value),
            Expr::Subscript(subscript) => {
                self.visit_expr(&subscript.value);
                self.visit_expr(&subscript.slice);
            }
            _ => {}
        }
    }

    fn record_walrus(&mut self, named: &ExprNamed) {
        self.visit_expr(&named.value);
        let Expr::Name(name) = &*named.target else {
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
        if let Expr::Name(_) = &*node.target {
            self.record_target(&node.target, BindingKind::Assignment);
        }
    }

    fn visit_arguments(&mut self, arguments: &ruff_python_ast::Arguments) {
        for arg in &arguments.args {
            self.visit_expr(arg);
        }
        for keyword in &arguments.keywords {
            self.visit_expr(&keyword.value);
        }
    }

    fn visit_assign(&mut self, node: &StmtAssign) {
        self.visit_expr(&node.value);
        for target in &node.targets {
            self.record_target(target, BindingKind::Assignment);
        }
    }

    fn visit_aug_assign(&mut self, node: &StmtAugAssign) {
        if let Expr::Name(name) = &*node.target {
            self.record_read(name.id.as_str(), name.range().start());
        }
        self.visit_expr(&node.value);
        if let Expr::Name(name) = &*node.target {
            self.record_write(
                name.id.as_str(),
                name.range().start(),
                BindingKind::AugAssign,
            );
        } else {
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

    fn walk_parameter_defaults(&mut self, parameters: &Parameters) {
        for parameter in parameters.iter_non_variadic_params() {
            if let Some(default) = &parameter.default {
                self.visit_expr(default);
            }
        }
    }
}

impl<'a> Visitor<'a> for Builder {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                if matches!(name.ctx, ExprContext::Load) {
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
fn top_level_module(dotted: &str) -> &str {
    dotted.split_once('.').map_or(dotted, |(head, _)| head)
}

#[cfg(test)]
mod tests {
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
    fn assignment_count_separates_initial_and_augmented() {
        let analysis = analyze("x = 0\nx += 1\n");
        let x = module_binding(&analysis, "x");
        assert_eq!(analysis.assignment_count(x), 2);
    }

    #[test]
    fn augmented_assignment_records_read_then_write() {
        let analysis = analyze("x = 0\nx += 1\n");
        let x = module_binding(&analysis, "x");
        assert_eq!(analysis.usage_count(x), 1);
        assert_eq!(analysis.assignment_count(x), 2);
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
    fn class_scope_is_invisible_to_nested_function() {
        let analysis = analyze("class C:\n    x = 1\n    def m(self):\n        return x\n");
        let class_scope = analysis
            .scopes
            .iter()
            .find(|s| matches!(s.kind, ScopeKind::Class))
            .expect("class scope exists");
        let x_in_class = *class_scope.bindings.get("x").expect("class binds x");
        assert_eq!(analysis.usage_count(x_in_class), 0);
    }

    #[test]
    fn closure_read_attributes_to_outer_binding() {
        let analysis = analyze("def outer():\n    x = 1\n    def inner():\n        return x\n");
        let outer_function_scope = &analysis.scopes[1];
        let x = *outer_function_scope
            .bindings
            .get("x")
            .expect("outer binds x");
        assert_eq!(analysis.usage_count(x), 1);
    }

    #[test]
    fn comprehension_target_stays_in_comprehension_scope() {
        let analysis = analyze("total = [x for x in xs]\n");
        let module = &analysis.scopes[0];
        assert!(module.bindings.contains_key("total"));
        assert!(!module.bindings.contains_key("x"));
    }

    #[test]
    fn except_handler_binding_lives_in_enclosing_scope() {
        let analysis = analyze("try:\n    f()\nexcept Exception as e:\n    print(e)\n");
        let e = module_binding(&analysis, "e");
        assert_eq!(analysis.usage_count(e), 1);
        assert_eq!(
            analysis.bindings[e.0 as usize].kinds,
            vec![BindingKind::ExceptHandler]
        );
    }

    #[test]
    fn import_binds_top_level_module_segment() {
        let analysis = analyze("import a.b.c\n");
        let module = &analysis.scopes[0];
        assert!(module.bindings.contains_key("a"));
        assert!(!module.bindings.contains_key("a.b.c"));
    }

    #[test]
    fn import_with_asname_binds_alias() {
        let analysis = analyze("import a.b.c as abc\n");
        let module = &analysis.scopes[0];
        assert!(module.bindings.contains_key("abc"));
        assert!(!module.bindings.contains_key("a"));
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
    fn lambda_parameters_shadow_outer_binding() {
        let analysis = analyze("x = 1\nf = lambda x: x\n");
        let lambda_scope = analysis
            .scopes
            .iter()
            .skip(1)
            .find(|s| matches!(s.kind, ScopeKind::Function))
            .expect("lambda creates function scope");
        let inner = *lambda_scope.bindings.get("x").expect("lambda binds x");
        assert_eq!(analysis.usage_count(inner), 1);
    }

    #[test]
    fn nonlocal_and_global_statements_are_inert() {
        let analysis = analyze("def f():\n    global x\n    x = 1\n");
        let module = &analysis.scopes[0];
        assert!(!module.bindings.contains_key("x"));
        let function_scope = &analysis.scopes[1];
        assert!(function_scope.bindings.contains_key("x"));
    }

    #[test]
    fn top_level_module_returns_first_segment() {
        assert_eq!(top_level_module("a"), "a");
        assert_eq!(top_level_module("a.b"), "a");
        assert_eq!(top_level_module("a.b.c"), "a");
        assert_eq!(top_level_module(""), "");
    }

    #[test]
    fn tuple_target_records_each_element() {
        let analysis = analyze("a, b = (1, 2)\n");
        let module = &analysis.scopes[0];
        assert!(module.bindings.contains_key("a"));
        assert!(module.bindings.contains_key("b"));
    }

    #[test]
    fn walrus_target_lifts_out_of_comprehension_scope() {
        let analysis = analyze("xs = [1]\nvals = [y for x in xs if (y := x + 1) > 0]\n");
        let module = &analysis.scopes[0];
        let y = *module
            .bindings
            .get("y")
            .expect("walrus binds y at module scope");
        assert_eq!(
            analysis.bindings[y.0 as usize].kinds,
            vec![BindingKind::Walrus]
        );
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
