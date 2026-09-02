//! The single walk that fills the binding table, pushing a scope frame
//! per `def`, `lambda`, `class`, and comprehension, recording every
//! write and read it meets, and resolving each deferred read against the
//! completed scope chain.

use std::{collections::BTreeSet, sync::OnceLock};

use indexmap::IndexMap;
use itertools::Itertools;
use ruff_python_ast::{
    CmpOp, ExceptHandler, Expr, ExprCompare, ExprDictComp, ExprGenerator, ExprLambda, ExprList,
    ExprListComp, ExprNamed, ExprSetComp, ExprTuple, Identifier, MatchCase, Operator, Parameters,
    Pattern, PatternMatchAs, PatternMatchMapping, PatternMatchStar, Stmt, StmtAnnAssign,
    StmtAssign, StmtAugAssign, StmtClassDef, StmtDelete, StmtFor, StmtFunctionDef, StmtGlobal,
    StmtIf, StmtImport, StmtImportFrom, StmtTry, StmtWhile, StmtWith, UnaryOp,
    name::Name,
    visitor::{Visitor, walk_arguments, walk_expr, walk_parameters, walk_pattern},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use super::{
    Binding, BindingAnalysis, BindingId, BindingKind, Scope, ScopeId, ScopeKind, UnpackKind,
    names::{bare_import_bound_name, from_import_bound_name},
};
use crate::primitives::{sorted_slot, walk::walk_stmt};

pub(super) struct Builder {
    annotation_depth: usize,
    annotation_offsets: FxHashSet<TextSize>,
    assignment_values: FxHashMap<TextSize, TextRange>,
    bindings: Vec<Binding>,
    conditional_depth: usize,
    deferred_reads: Vec<DeferredRead>,
    deleted: FxHashSet<Name>,
    function_scope_at: FxHashMap<TextSize, ScopeId>,
    global_writes: FxHashMap<Name, Vec<TextSize>>,
    runtime_offsets: FxHashSet<TextSize>,
    scope_stack: Vec<ScopeId>,
    scopes: Vec<Scope>,
    unpack_groups: Vec<UnpackGroup>,
}

impl Builder {
    pub(super) fn new() -> Self {
        let mut builder = Self {
            annotation_depth: 0,
            annotation_offsets: FxHashSet::default(),
            assignment_values: FxHashMap::default(),
            bindings: Vec::new(),
            conditional_depth: 0,
            deferred_reads: Vec::new(),
            deleted: FxHashSet::default(),
            function_scope_at: FxHashMap::default(),
            global_writes: FxHashMap::default(),
            runtime_offsets: FxHashSet::default(),
            scope_stack: Vec::new(),
            scopes: Vec::new(),
            unpack_groups: Vec::new(),
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
        if let Some(type_params) = &class.type_params {
            self.in_annotation(|b| b.visit_type_params(type_params));
        }
        if let Some(arguments) = &class.arguments {
            walk_arguments(self, arguments);
        }
        self.record_identifier(&class.name, BindingKind::ClassDef);
        self.in_scope(ScopeKind::Class, |b| b.visit_body(&class.body));
    }

    fn enter_comprehension(
        &mut self,
        generators: &[ruff_python_ast::Comprehension],
        elements: &[&Expr],
    ) {
        let Some(first) = generators.first() else {
            unreachable!(
                "invariant: comprehension carries at least one generator (parser guarantee)"
            );
        };
        self.visit_expr(&first.iter);
        self.in_scope(ScopeKind::Comprehension, |b| {
            for (index, generator) in generators.iter().enumerate() {
                if index > 0 {
                    b.visit_expr(&generator.iter);
                }
                b.record_target(&generator.target, BindingKind::Comprehension);
                for guard in &generator.ifs {
                    b.visit_expr(guard);
                }
            }
            for element in elements {
                b.visit_expr(element);
            }
        });
    }

    fn enter_function(&mut self, function: &StmtFunctionDef, stmt_start: TextSize) {
        for decorator in &function.decorator_list {
            self.visit_expr(&decorator.expression);
        }
        if let Some(type_params) = &function.type_params {
            self.in_annotation(|b| b.visit_type_params(type_params));
        }
        walk_parameters(self, &function.parameters);
        if let Some(returns) = &function.returns {
            self.visit_annotation(returns);
        }
        self.record_identifier(&function.name, BindingKind::FunctionDef);
        let function_scope = self.in_scope(ScopeKind::Function, |b| {
            b.record_parameters(&function.parameters);
            b.visit_body(&function.body);
        });
        self.function_scope_at.insert(stmt_start, function_scope);
    }

    fn enter_lambda(&mut self, lambda: &ExprLambda) {
        if let Some(parameters) = &lambda.parameters {
            walk_parameters(self, parameters);
        }
        self.in_scope(ScopeKind::Function, |b| {
            if let Some(parameters) = &lambda.parameters {
                b.record_parameters(parameters);
            }
            b.visit_expr(&lambda.body);
        });
    }

    fn for_each_target_name(
        &mut self,
        target: &Expr,
        f: &mut impl FnMut(&mut Self, &str, TextSize),
    ) {
        match target {
            Expr::Name(name) => f(self, name.id.as_str(), name.start()),
            Expr::Tuple(ExprTuple { elts, .. }) | Expr::List(ExprList { elts, .. }) => {
                for element in elts {
                    self.for_each_target_name(element, f);
                }
            }
            Expr::Starred(starred) => self.for_each_target_name(&starred.value, f),
            _ => walk_expr(self, target),
        }
    }

    /// Runs `f` with annotation depth raised, so every read it reaches
    /// records as a type use.
    fn in_annotation(&mut self, f: impl FnOnce(&mut Self)) {
        self.annotation_depth += 1;
        f(self);
        self.annotation_depth -= 1;
    }

    /// Runs `f` with writes marked conditional, so a name bound only
    /// inside a branch that may not run never sets
    /// `first_unconditional_write`.
    fn in_conditional(&mut self, f: impl FnOnce(&mut Self)) {
        self.conditional_depth += 1;
        f(self);
        self.conditional_depth -= 1;
    }

    /// Runs `f` inside a freshly pushed scope of `kind` parented to the
    /// current scope, popping it when `f` returns. Returns the new
    /// scope's id for a caller that records it.
    fn in_scope(&mut self, kind: ScopeKind, f: impl FnOnce(&mut Self)) -> ScopeId {
        let parent = Some(self.current_scope());
        let id = self.push_scope(kind, parent);
        f(self);
        self.pop_scope();
        id
    }

    /// Marks each operand an order comparison ranks (`{open, stat} <=
    /// supports_dir_fd`).
    fn mark_comparison(&mut self, node: &ExprCompare) {
        let operands = std::iter::once(node.left.as_ref()).chain(&node.comparators);
        for ((left, right), op) in operands.tuple_windows().zip(&node.ops) {
            if matches!(op, CmpOp::Gt | CmpOp::GtE | CmpOp::Lt | CmpOp::LtE) {
                self.mark_runtime_read(left);
                self.mark_runtime_read(right);
            }
        }
    }

    /// Marks every operand of `expr` that stands where only data stands.
    /// A class raises `TypeError` on each of these, whereas it iterates,
    /// compares by equality, and answers `is` like any other object.
    fn mark_runtime_operands(&mut self, expr: &Expr) {
        match expr {
            Expr::BinOp(node) if node.op != Operator::BitOr => {
                self.mark_runtime_read(&node.left);
                self.mark_runtime_read(&node.right);
            }
            Expr::BoolOp(node) => {
                for value in &node.values {
                    self.mark_runtime_read(value);
                }
            }
            Expr::Compare(node) => self.mark_comparison(node),
            Expr::If(node) => self.mark_runtime_read(&node.test),
            Expr::UnaryOp(node) if node.op == UnaryOp::Not => {
                self.mark_runtime_read(&node.operand);
            }
            _ => {}
        }
    }

    /// Records `expr` as read where only a runtime object stands. Only a
    /// bare name marks, in that `if f(X):` reads `X` as a call argument
    /// and settles nothing, whereas `if X:` truth-tests the object
    /// itself. An annotation never marks, because a type stands there.
    fn mark_runtime_read(&mut self, expr: &Expr) {
        if self.annotation_depth > 0 {
            return;
        }
        if let Expr::Name(name) = expr
            && name.ctx.is_load()
        {
            self.runtime_offsets.insert(name.start());
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
            bindings: IndexMap::default(),
            globals: FxHashSet::default(),
        });
        self.scope_stack.push(id);
        id
    }

    fn record_attribute_read(&mut self, name: &str, offset: TextSize, attribute: &str) {
        self.record_use(name, offset, Some(attribute));
    }

    fn record_identifier(&mut self, identifier: &Identifier, kind: BindingKind) {
        self.record_write(identifier.as_str(), identifier.start(), kind);
    }

    fn record_parameters(&mut self, parameters: &Parameters) {
        for parameter in parameters.iter_source_order() {
            self.record_identifier(parameter.name(), BindingKind::Parameter);
        }
    }

    fn record_read(&mut self, name: &str, offset: TextSize) {
        self.record_use(name, offset, None);
    }

    /// Records a read of `id` at `offset`, inserting into `read_offsets`
    /// so they stay ascending whether the read arrives in source order
    /// or as a deferred forward reference. Flags the read bare or under
    /// the attribute it accessed.
    fn record_resolved_read(&mut self, id: BindingId, offset: TextSize, attribute: Option<&str>) {
        let binding = &mut self.bindings[id.0 as usize];
        let slot = sorted_slot(&binding.read_offsets, &offset, |&existing| existing);
        binding.read_offsets.insert(slot, offset);
        match attribute {
            Some(attribute) => {
                binding.attributes.insert(attribute.into());
            }
            None => binding.bare_read = true,
        }
    }

    fn record_target(&mut self, target: &Expr, kind: BindingKind) {
        self.for_each_target_name(target, &mut |builder, name, offset| {
            builder.record_write(name, offset, kind);
        });
    }

    fn record_unpack(&mut self, elts: &[Expr], value: &Expr) {
        let mut members = Vec::new();
        for element in elts {
            self.for_each_target_name(element, &mut |builder, name, offset| {
                members.push(builder.record_write(name, offset, BindingKind::Assignment));
            });
        }
        if members.len() < 2 {
            return;
        }
        let suggestible = elts.iter().all(Expr::is_name_expr)
            && (value.is_name_expr() || value.is_attribute_expr());
        self.unpack_groups.push(UnpackGroup {
            members,
            suggestible,
            value: value.range(),
        });
    }

    fn record_use(&mut self, name: &str, offset: TextSize, attribute: Option<&str>) {
        if self.annotation_depth > 0 {
            self.annotation_offsets.insert(offset);
        }
        let innermost = self.current_scope();
        match resolve_in_chain(&self.scopes, innermost, name) {
            Some(binding) => self.record_resolved_read(binding, offset, attribute),
            None => self.deferred_reads.push(DeferredRead {
                attribute: attribute.map(Name::from),
                name: name.into(),
                offset,
                scope: innermost,
            }),
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
        let binding =
            self.record_write_in(scope, name.id.as_str(), name.start(), BindingKind::Walrus);
        self.record_resolved_read(binding, name.start(), None);
    }

    fn record_write(&mut self, name: &str, offset: TextSize, kind: BindingKind) -> BindingId {
        let scope = self.current_scope();
        if self.scopes[scope.0 as usize].globals.contains(name) {
            self.global_writes
                .entry(name.into())
                .or_default()
                .push(offset);
            return self.record_write_in(ScopeId(0), name, offset, kind);
        }
        self.record_write_in(scope, name, offset, kind)
    }

    fn record_write_in(
        &mut self,
        scope: ScopeId,
        name: &str,
        offset: TextSize,
        kind: BindingKind,
    ) -> BindingId {
        let unconditional = self.conditional_depth == 0;
        let scope_data = &mut self.scopes[scope.0 as usize];
        let binding_id = if let Some(&id) = scope_data.bindings.get(name) {
            id
        } else {
            let id =
                BindingId(u32::try_from(self.bindings.len()).expect("binding count fits in u32"));
            scope_data.bindings.insert(name.into(), id);
            self.bindings.push(Binding {
                annotation_read: false,
                attributes: BTreeSet::new(),
                bare_read: false,
                first_unconditional_write: None,
                kinds: SmallVec::new(),
                name: name.into(),
                read_offsets: SmallVec::new(),
                runtime_read: false,
                scope,
                write_offsets: SmallVec::new(),
            });
            id
        };
        let binding = &mut self.bindings[binding_id.0 as usize];
        if !binding.kinds.contains(&kind) {
            binding.kinds.push(kind);
        }
        binding.write_offsets.push(offset);
        if unconditional {
            binding.first_unconditional_write.get_or_insert(offset);
        }
        binding_id
    }

    fn visit_ann_assign(&mut self, node: &StmtAnnAssign) {
        self.visit_annotation(&node.annotation);
        if let Some(value) = &node.value {
            self.visit_expr(value);
        }
        match node.target.as_ref() {
            Expr::Name(name) => {
                if let Some(value) = &node.value {
                    self.assignment_values.insert(name.start(), value.range());
                }
                self.record_target(&node.target, BindingKind::Assignment);
            }
            target => walk_expr(self, target),
        }
    }

    fn visit_assign(&mut self, node: &StmtAssign) {
        self.visit_expr(&node.value);
        for target in &node.targets {
            if let Expr::Name(name) = target {
                self.assignment_values
                    .insert(name.start(), node.value.range());
            }
            match target {
                Expr::Tuple(ExprTuple { elts, .. }) | Expr::List(ExprList { elts, .. }) => {
                    self.record_unpack(elts, &node.value);
                }
                _ => self.record_target(target, BindingKind::Assignment),
            }
        }
    }

    fn visit_aug_assign(&mut self, node: &StmtAugAssign) {
        if let Some(name) = node.target.as_name_expr() {
            self.record_read(name.id.as_str(), name.start());
            self.visit_expr(&node.value);
            self.record_write(name.id.as_str(), name.start(), BindingKind::AugAssign);
        } else {
            self.visit_expr(&node.value);
            walk_expr(self, &node.target);
        }
    }

    /// Visits `test` as the condition of an `if`, `elif`, or `while`,
    /// recording its bare name as a runtime read.
    fn visit_condition_test(&mut self, test: &Expr) {
        self.mark_runtime_read(test);
        self.visit_expr(test);
    }

    /// Records each name a `del` unbinds, which is neither a read nor a
    /// write, and walks any other target shape for its own reads.
    fn visit_delete(&mut self, node: &StmtDelete) {
        for target in &node.targets {
            match target {
                Expr::Name(name) => {
                    self.deleted.insert(name.id.clone());
                }
                _ => walk_expr(self, target),
            }
        }
    }

    fn visit_for(&mut self, node: &StmtFor) {
        self.visit_expr(&node.iter);
        self.in_conditional(|b| {
            b.record_target(&node.target, BindingKind::For);
            b.visit_body(&node.body);
            b.visit_body(&node.orelse);
        });
    }

    /// Records each name a `global` statement declares, so a later
    /// write in this scope binds at module scope rather than locally.
    fn visit_global(&mut self, node: &StmtGlobal) {
        let scope = self.current_scope();
        for name in &node.names {
            self.scopes[scope.0 as usize]
                .globals
                .insert(name.id.clone());
        }
    }

    /// Walks an `if`/`elif`/`else` chain with each branch body conditional.
    fn visit_if(&mut self, node: &StmtIf) {
        self.visit_condition_test(&node.test);
        self.in_conditional(|b| b.visit_body(&node.body));
        for clause in &node.elif_else_clauses {
            if let Some(test) = &clause.test {
                self.visit_condition_test(test);
            }
            self.in_conditional(|b| b.visit_body(&clause.body));
        }
    }

    fn visit_import(&mut self, node: &StmtImport) {
        for alias in &node.names {
            let bound = bare_import_bound_name(alias);
            self.record_write(bound, alias.start(), BindingKind::Import);
        }
    }

    fn visit_import_from(&mut self, node: &StmtImportFrom) {
        for alias in &node.names {
            let bound = from_import_bound_name(alias);
            self.record_write(bound, alias.start(), BindingKind::Import);
        }
    }

    fn visit_try(&mut self, node: &StmtTry) {
        self.in_conditional(|b| {
            b.visit_body(&node.body);
            for handler in &node.handlers {
                let ExceptHandler::ExceptHandler(eh) = handler;
                if let Some(type_) = &eh.type_ {
                    b.visit_expr(type_);
                }
                if let Some(name) = &eh.name {
                    b.record_identifier(name, BindingKind::ExceptHandler);
                }
                b.visit_body(&eh.body);
            }
            b.visit_body(&node.orelse);
        });
        self.visit_body(&node.finalbody);
    }

    fn visit_while(&mut self, node: &StmtWhile) {
        self.visit_condition_test(&node.test);
        self.in_conditional(|b| {
            b.visit_body(&node.body);
            b.visit_body(&node.orelse);
        });
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

    pub(super) fn finish(mut self) -> BindingAnalysis {
        for deferred in std::mem::take(&mut self.deferred_reads) {
            if let Some(binding_id) = resolve_in_chain(&self.scopes, deferred.scope, &deferred.name)
            {
                self.record_resolved_read(
                    binding_id,
                    deferred.offset,
                    deferred.attribute.as_deref(),
                );
            }
        }
        // Folded after the deferred reads land, so a forward reference
        // from an annotation to a name bound later still records.
        for binding in &mut self.bindings {
            binding.annotation_read = binding
                .read_offsets
                .iter()
                .any(|offset| self.annotation_offsets.contains(offset));
            binding.runtime_read = binding
                .read_offsets
                .iter()
                .any(|offset| self.runtime_offsets.contains(offset));
        }
        let mut unpack_targets = FxHashMap::default();
        for group in &self.unpack_groups {
            let reused = group
                .members
                .iter()
                .any(|&member| self.bindings[member.0 as usize].read_offsets.len() > 1);
            for (index, &member) in group.members.iter().enumerate() {
                let kind = if !reused && group.suggestible {
                    UnpackKind::Suggested(group.value, index)
                } else {
                    UnpackKind::Unresolved
                };
                unpack_targets.insert(member, kind);
            }
        }
        BindingAnalysis {
            assignment_values: self.assignment_values,
            bindings: self.bindings,
            deleted: self.deleted,
            function_scope_at: self.function_scope_at,
            global_writes: self.global_writes,
            module_reads: OnceLock::new(),
            scopes: self.scopes,
            unpack_targets,
        }
    }
}

impl<'a> Visitor<'a> for Builder {
    fn visit_annotation(&mut self, expr: &'a Expr) {
        self.in_annotation(|b| b.visit_expr(expr));
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.mark_runtime_operands(expr);
        match expr {
            Expr::Name(name) => {
                if name.ctx.is_load() {
                    self.record_read(name.id.as_str(), name.start());
                }
            }
            Expr::Attribute(attr) => match attr.value.as_ref() {
                Expr::Name(name) if name.ctx.is_load() => {
                    self.record_attribute_read(name.id.as_str(), name.start(), attr.attr.as_str())
                }
                _ => walk_expr(self, expr),
            },
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
            }) => match key {
                Some(key) => self.enter_comprehension(generators, &[key, value]),
                None => self.enter_comprehension(generators, &[value]),
            },
            _ => walk_expr(self, expr),
        }
    }

    /// Records the name a capture pattern binds, which the upstream
    /// walk reaches as an identifier rather than an expression.
    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        match pattern {
            Pattern::MatchAs(PatternMatchAs {
                name: Some(name), ..
            })
            | Pattern::MatchStar(PatternMatchStar {
                name: Some(name), ..
            })
            | Pattern::MatchMapping(PatternMatchMapping {
                rest: Some(name), ..
            }) => self.record_identifier(name, BindingKind::Assignment),
            _ => {}
        }
        walk_pattern(self, pattern);
    }

    fn visit_match_case(&mut self, case: &'a MatchCase) {
        self.visit_pattern(&case.pattern);
        if let Some(guard) = &case.guard {
            self.visit_expr(guard);
        }
        self.in_conditional(|b| b.visit_body(&case.body));
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::AnnAssign(node) => self.visit_ann_assign(node),
            Stmt::Assign(node) => self.visit_assign(node),
            Stmt::AugAssign(node) => self.visit_aug_assign(node),
            Stmt::ClassDef(node) => self.enter_class(node),
            Stmt::Delete(node) => self.visit_delete(node),
            Stmt::For(node) => self.visit_for(node),
            Stmt::FunctionDef(node) => self.enter_function(node, stmt.start()),
            Stmt::Global(node) => self.visit_global(node),
            Stmt::Nonlocal(_) => {}
            Stmt::If(node) => self.visit_if(node),
            Stmt::Import(node) => self.visit_import(node),
            Stmt::ImportFrom(node) => self.visit_import_from(node),
            Stmt::Try(node) => self.visit_try(node),
            // `walk_stmt` reaches the value, the type parameters, and the
            // name, so a PEP 695 bound (`type R[T: abc.Mapping] = ...`)
            // records its reads. Every one of them is a type position.
            Stmt::TypeAlias(_) => self.in_annotation(|b| walk_stmt(b, stmt)),
            Stmt::While(node) => self.visit_while(node),
            Stmt::With(node) => self.visit_with(node),
            _ => walk_stmt(self, stmt),
        }
    }
}

/// A read left unresolved mid-walk, retained until `finish`
/// re-resolves it against the completed scope chain.
struct DeferredRead {
    attribute: Option<Name>,
    name: Name,
    offset: TextSize,
    scope: ScopeId,
}

/// One multi-name unpack assignment, retained until `finish` reads the
/// final sibling read counts.
struct UnpackGroup {
    members: Vec<BindingId>,
    suggestible: bool,
    value: TextRange,
}

/// Resolves `name` against the scope chain rooted at `innermost`,
/// walking outward through `parent` links. A non-innermost class scope
/// is skipped, since its names are invisible to nested functions and
/// comprehensions. `None` when no scope in the chain binds `name`.
fn resolve_in_chain(scopes: &[Scope], innermost: ScopeId, name: &str) -> Option<BindingId> {
    std::iter::successors(Some(innermost), |&id| scopes[id.0 as usize].parent)
        .filter(|&id| id == innermost || !matches!(scopes[id.0 as usize].kind, ScopeKind::Class))
        .find_map(|id| scopes[id.0 as usize].bindings.get(name).copied())
}
