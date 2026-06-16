//! Per-`Source` binding-resolution table.
//!
//! Walks the module once and records, for every name introduced or
//! shadowed in a lexical scope, the offsets of every write and read.
//! A read that finds no binding mid-walk defers and resolves against
//! the completed scope chain after the walk, so a forward reference to
//! a name bound later in source order still records against it.
//! Consuming rules query the table by `BindingId`, by name, by source
//! offset, or by an owning `&Stmt` rather than driving their own walk.
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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use ruff_python_ast::{
    ExceptHandler, Expr, ExprDictComp, ExprGenerator, ExprLambda, ExprList, ExprListComp,
    ExprNamed, ExprSetComp, ExprTuple, Identifier, MatchCase, ModModule, Parameters, Stmt,
    StmtAnnAssign, StmtAssign, StmtAugAssign, StmtClassDef, StmtFor, StmtFunctionDef, StmtIf,
    StmtImport, StmtImportFrom, StmtTry, StmtWhile, StmtWith,
    visitor::{Visitor, walk_arguments, walk_expr, walk_parameters, walk_stmt},
};
use ruff_text_size::{Ranged, TextRange, TextSize};
use serde::Serialize;

mod names;

pub(crate) use names::{
    annotated_name_target, bare_import_bound_name, from_import_bound_name, single_name_target,
    tail_identifier, top_level_module,
};

/// Stable handle to a binding in `BindingAnalysis`. Cheap to copy.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct BindingId(u32);

/// Stable handle to a scope in `BindingAnalysis`. Cheap to copy.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
struct ScopeId(u32);

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
enum ScopeKind {
    Class,
    Comprehension,
    Function,
    Module,
}

/// Disposition of a multi-name unpack target for the single-use lint.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum UnpackKind {
    /// Flagged with no subscript rewrite, because the right-hand side
    /// is a call or a starred target shifts the indices.
    Bare,
    /// A sibling target reads more than once, so removing this target
    /// would split the unpack into an indexed read.
    Exempt,
    /// Flagged with a subscript rewrite: the right-hand-side range and
    /// this target's index.
    Suggested(TextRange, usize),
}

/// One named binding in some scope, with every observed write and read.
/// `attributes` collects the distinct attribute names read off the
/// binding (`os.environ` records `environ`), `bare_read` flips when the
/// name is read without an attribute access (`foo(os)`), and
/// `first_unconditional_write` holds the earliest write not nested in a
/// conditional branch (`if`/`for`/`while`/`try`/`match`), or `None` when
/// every write is conditional.
#[derive(Debug, Serialize)]
struct Binding {
    attributes: BTreeSet<String>,
    bare_read: bool,
    first_unconditional_write: Option<TextSize>,
    kinds: Vec<BindingKind>,
    name: String,
    read_offsets: Vec<TextSize>,
    scope: ScopeId,
    write_offsets: Vec<TextSize>,
}

/// One lexical scope plus its binding table keyed by name.
#[derive(Debug, Serialize)]
struct Scope {
    bindings: BTreeMap<String, BindingId>,
    kind: ScopeKind,
    parent: Option<ScopeId>,
}

/// Module-wide binding-resolution table.
#[derive(Debug, Serialize)]
pub struct BindingAnalysis {
    #[serde(skip)]
    assignment_values: HashMap<TextSize, TextRange>,
    bindings: Vec<Binding>,
    #[serde(skip)]
    condition_test_walruses: HashSet<BindingId>,
    #[serde(skip)]
    function_scope_at: HashMap<TextSize, ScopeId>,
    scopes: Vec<Scope>,
    #[serde(skip)]
    unpack_targets: HashMap<BindingId, UnpackKind>,
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

    fn module_binding(&self, name: &str) -> Option<&Binding> {
        self.scopes[0]
            .bindings
            .get(name)
            .map(|&id| self.binding(id))
    }

    /// Returns the number of write events recorded for `binding`.
    pub(crate) fn assignment_count(&self, binding: BindingId) -> usize {
        self.binding(binding).write_offsets.len()
    }

    /// Returns the source range of the value bound at `offset`, for a
    /// direct `name = value` or `name: T = value` write. `None` for a
    /// tuple/list target, a bare annotation, or an unrecorded offset.
    pub(crate) fn assignment_value_range(&self, offset: TextSize) -> Option<TextRange> {
        self.assignment_values.get(&offset).copied()
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
    /// scope of `stmt`. `stmt` must be a `Stmt::FunctionDef`. Any
    /// other statement yields an empty iterator.
    pub(crate) fn bindings_in_scope(
        &self,
        stmt: &Stmt,
    ) -> impl Iterator<Item = BindingId> + use<'_> {
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

    /// Returns `true` when `name` has an unconditional module-scope
    /// write at an offset strictly less than `offset`. A write nested in
    /// a conditional branch (`if`/`for`/`while`/`try`/`match`) is excluded.
    pub(crate) fn is_defined_before(&self, name: &str, offset: TextSize) -> bool {
        self.module_binding(name)
            .and_then(|binding| binding.first_unconditional_write)
            .is_some_and(|first| first < offset)
    }

    /// Returns the number of distinct attributes read off the
    /// module-scope binding for `name` (`os.environ` and `os.getcwd`
    /// count as two), or `0` when `name` is unbound at module scope.
    pub(crate) fn module_attribute_count(&self, name: &str) -> usize {
        self.module_binding(name)
            .map_or(0, |binding| binding.attributes.len())
    }

    /// Returns the read offsets of the module-scope binding for `name`
    /// when its sole write is one function definition. `None` when
    /// `name` is unbound at module scope, rebound, written by anything
    /// other than a single `def`, or potentially rebound by a
    /// module-scope `from x import *`.
    pub(crate) fn module_function_reads(&self, name: &str) -> Option<&[TextSize]> {
        // A `from x import *` binds under `*` rather than under each
        // real name it pulls in, so the visible `def` may not be the
        // function the call actually reaches, leaving no module name
        // safe to resolve against.
        if self.module_binding("*").is_some() {
            return None;
        }
        let binding = self.module_binding(name)?;
        (binding.kinds == [BindingKind::FunctionDef] && binding.write_offsets.len() == 1)
            .then_some(binding.read_offsets.as_slice())
    }

    /// Returns `true` when the module-scope binding for `name` carries
    /// more than one write or an augmented-assignment write, and
    /// `false` when `name` is write-once or unbound at module scope.
    pub(crate) fn module_reassigned(&self, name: &str) -> bool {
        self.module_binding(name).is_some_and(|binding| {
            binding.write_offsets.len() > 1 || binding.kinds.contains(&BindingKind::AugAssign)
        })
    }

    /// Returns `true` when the module-scope binding for `name` is read
    /// without an attribute access anywhere (the namespace object
    /// itself is used), and `false` when `name` is only attribute-read
    /// or unbound at module scope.
    pub(crate) fn module_used_bare(&self, name: &str) -> bool {
        self.module_binding(name)
            .is_some_and(|binding| binding.bare_read)
    }

    /// Returns the unpack disposition of `binding` when its sole write
    /// is a multi-name tuple or list unpack target, `None` otherwise.
    pub(crate) fn unpack_target(&self, binding: BindingId) -> Option<UnpackKind> {
        self.unpack_targets.get(&binding).copied()
    }

    /// Returns the number of read events recorded for `binding`.
    pub(crate) fn usage_count(&self, binding: BindingId) -> usize {
        self.binding(binding).read_offsets.len()
    }

    /// Returns `true` when a walrus write of `binding` occurred in the
    /// test of an `if`, `elif`, or `while`.
    pub(crate) fn walrus_in_condition(&self, binding: BindingId) -> bool {
        self.condition_test_walruses.contains(&binding)
    }
}

struct Builder {
    assignment_values: HashMap<TextSize, TextRange>,
    bindings: Vec<Binding>,
    condition_test_depth: usize,
    condition_test_walruses: HashSet<BindingId>,
    conditional_depth: usize,
    deferred_reads: Vec<DeferredRead>,
    function_scope_at: HashMap<TextSize, ScopeId>,
    scope_stack: Vec<ScopeId>,
    scopes: Vec<Scope>,
    unpack_groups: Vec<UnpackGroup>,
}

impl Builder {
    fn new() -> Self {
        let mut builder = Self {
            assignment_values: HashMap::new(),
            bindings: Vec::new(),
            condition_test_depth: 0,
            condition_test_walruses: HashSet::new(),
            conditional_depth: 0,
            deferred_reads: Vec::new(),
            function_scope_at: HashMap::new(),
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
        let Some((first, rest)) = generators.split_first() else {
            unreachable!(
                "invariant: comprehension carries at least one generator (parser guarantee)"
            );
        };
        self.visit_expr(&first.iter);
        self.in_scope(ScopeKind::Comprehension, |b| {
            b.record_target(&first.target, BindingKind::Comprehension);
            for guard in &first.ifs {
                b.visit_expr(guard);
            }
            for generator in rest {
                b.visit_expr(&generator.iter);
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
        walk_parameters(self, &function.parameters);
        if let Some(returns) = &function.returns {
            self.visit_expr(returns);
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

    fn finish(mut self) -> BindingAnalysis {
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
        let mut unpack_targets = HashMap::new();
        for group in &self.unpack_groups {
            let reused = group
                .members
                .iter()
                .any(|&member| self.bindings[member.0 as usize].read_offsets.len() > 1);
            for (index, &member) in group.members.iter().enumerate() {
                let kind = if reused {
                    UnpackKind::Exempt
                } else if group.suggestible {
                    UnpackKind::Suggested(group.value, index)
                } else {
                    UnpackKind::Bare
                };
                unpack_targets.insert(member, kind);
            }
        }
        BindingAnalysis {
            assignment_values: self.assignment_values,
            bindings: self.bindings,
            condition_test_walruses: self.condition_test_walruses,
            function_scope_at: self.function_scope_at,
            scopes: self.scopes,
            unpack_targets,
        }
    }

    fn for_each_target_name(
        &mut self,
        target: &Expr,
        f: &mut impl FnMut(&mut Self, &str, TextSize),
    ) {
        match target {
            Expr::Name(name) => f(self, name.id.as_str(), name.range().start()),
            Expr::Tuple(ExprTuple { elts, .. }) | Expr::List(ExprList { elts, .. }) => {
                for element in elts {
                    self.for_each_target_name(element, f);
                }
            }
            Expr::Starred(starred) => self.for_each_target_name(&starred.value, f),
            _ => walk_expr(self, target),
        }
    }

    /// Runs `f` with condition-test depth raised, so a `:=` reached
    /// while visiting an `if`/`elif`/`while` test records into
    /// `condition_test_walruses`.
    fn in_condition_test(&mut self, f: impl FnOnce(&mut Self)) {
        self.condition_test_depth += 1;
        f(self);
        self.condition_test_depth -= 1;
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

    fn record_attribute_read(&mut self, name: &str, offset: TextSize, attribute: &str) {
        self.record_use(name, offset, Some(attribute));
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
        self.record_use(name, offset, None);
    }

    /// Records a read of `id` at `offset`, inserting into `read_offsets`
    /// so they stay ascending whether the read arrives in source order
    /// or as a deferred forward reference. Flags the read bare or under
    /// the attribute it accessed.
    fn record_resolved_read(&mut self, id: BindingId, offset: TextSize, attribute: Option<&str>) {
        let binding = &mut self.bindings[id.0 as usize];
        let slot = binding
            .read_offsets
            .partition_point(|&existing| existing < offset);
        binding.read_offsets.insert(slot, offset);
        match attribute {
            Some(attribute) => {
                binding.attributes.insert(attribute.to_owned());
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
        let innermost = self.current_scope();
        match resolve_in_chain(&self.scopes, innermost, name) {
            Some(binding) => self.record_resolved_read(binding, offset, attribute),
            None => self.deferred_reads.push(DeferredRead {
                attribute: attribute.map(str::to_owned),
                name: name.to_owned(),
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
        let binding = self.record_write_in(
            scope,
            name.id.as_str(),
            name.range().start(),
            BindingKind::Walrus,
        );
        if self.condition_test_depth > 0 {
            self.condition_test_walruses.insert(binding);
        }
    }

    fn record_write(&mut self, name: &str, offset: TextSize, kind: BindingKind) -> BindingId {
        let scope = self.current_scope();
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
            scope_data.bindings.insert(name.to_owned(), id);
            self.bindings.push(Binding {
                name: name.to_owned(),
                scope,
                attributes: BTreeSet::new(),
                bare_read: false,
                first_unconditional_write: None,
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
        if unconditional {
            binding.first_unconditional_write.get_or_insert(offset);
        }
        binding_id
    }

    fn visit_ann_assign(&mut self, node: &StmtAnnAssign) {
        self.visit_expr(&node.annotation);
        if let Some(value) = &node.value {
            self.visit_expr(value);
        }
        if let Expr::Name(name) = node.target.as_ref() {
            if let Some(value) = &node.value {
                self.assignment_values
                    .insert(name.range().start(), value.range());
            }
            self.record_target(&node.target, BindingKind::Assignment);
        }
    }

    fn visit_assign(&mut self, node: &StmtAssign) {
        self.visit_expr(&node.value);
        for target in &node.targets {
            if let Expr::Name(name) = target {
                self.assignment_values
                    .insert(name.range().start(), node.value.range());
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
        self.in_conditional(|b| {
            b.record_target(&node.target, BindingKind::For);
            b.visit_body(&node.body);
            b.visit_body(&node.orelse);
        });
    }

    /// Walks an `if`/`elif`/`else` chain with each branch body conditional.
    fn visit_if(&mut self, node: &StmtIf) {
        self.in_condition_test(|b| b.visit_expr(&node.test));
        self.in_conditional(|b| b.visit_body(&node.body));
        for clause in &node.elif_else_clauses {
            if let Some(test) = &clause.test {
                self.in_condition_test(|b| b.visit_expr(test));
            }
            self.in_conditional(|b| b.visit_body(&clause.body));
        }
    }

    fn visit_import(&mut self, node: &StmtImport) {
        for alias in &node.names {
            let bound = bare_import_bound_name(alias);
            self.record_write(bound, alias.range().start(), BindingKind::Import);
        }
    }

    fn visit_import_from(&mut self, node: &StmtImportFrom) {
        for alias in &node.names {
            let bound = from_import_bound_name(alias);
            self.record_write(bound, alias.range().start(), BindingKind::Import);
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
        self.in_condition_test(|b| b.visit_expr(&node.test));
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
}

impl<'a> Visitor<'a> for Builder {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                if name.ctx.is_load() {
                    self.record_read(name.id.as_str(), name.range().start());
                }
            }
            Expr::Attribute(attr) => match attr.value.as_ref() {
                Expr::Name(name) if name.ctx.is_load() => self.record_attribute_read(
                    name.id.as_str(),
                    name.range().start(),
                    attr.attr.as_str(),
                ),
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
            }) => self.enter_comprehension(generators, &[key, value]),
            _ => walk_expr(self, expr),
        }
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
            Stmt::For(node) => self.visit_for(node),
            Stmt::FunctionDef(node) => self.enter_function(node, stmt.range().start()),
            Stmt::Global(_) | Stmt::Nonlocal(_) => {}
            Stmt::If(node) => self.visit_if(node),
            Stmt::Import(node) => self.visit_import(node),
            Stmt::ImportFrom(node) => self.visit_import_from(node),
            Stmt::Try(node) => self.visit_try(node),
            Stmt::While(node) => self.visit_while(node),
            Stmt::With(node) => self.visit_with(node),
            _ => walk_stmt(self, stmt),
        }
    }
}

/// A read left unresolved mid-walk, retained until `finish`
/// re-resolves it against the completed scope chain.
struct DeferredRead {
    attribute: Option<String>,
    name: String,
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

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use proptest::prelude::*;
    use rstest::rstest;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::testing::parse;

    fn analyze(src: &str) -> BindingAnalysis {
        BindingAnalysis::new(parse(src).ast())
    }

    fn module_binding_id(analysis: &BindingAnalysis, name: &str) -> BindingId {
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
    fn deferred_read_resolves_to_an_enclosing_function_local() {
        let analysis = analyze(
            "def outer():\n    def inner():\n        return helper()\n    def helper():\n        return 1\n",
        );
        let outer = analysis
            .scopes
            .iter()
            .find(|scope| scope.parent == Some(ScopeId(0)))
            .expect("outer is the function scope under module");
        let helper = *outer.bindings.get("helper").expect("helper bound in outer");
        assert_eq!(
            analysis.usage_count(helper),
            1,
            "the forward call resolves to outer's local",
        );
    }

    #[rstest]
    #[case::conditional_only_write("if flag:\n    Helper = int\n", "Helper", 100)]
    #[case::elif_only_write("if a:\n    pass\nelif b:\n    Helper = int\n", "Helper", 100)]
    #[case::except_only_write("try:\n    pass\nexcept E:\n    Helper = int\n", "Helper", 100)]
    #[case::for_only_write("for _ in xs:\n    Helper = int\n", "Helper", 100)]
    #[case::match_case_only_write("match x:\n    case 1:\n        Helper = int\n", "Helper", 100)]
    #[case::nested_conditional("if a:\n    if b:\n        Helper = int\n", "Helper", 100)]
    #[case::try_only_write("try:\n    Helper = int\nexcept E:\n    pass\n", "Helper", 100)]
    #[case::undefined_name("x = 1\n", "y", 100)]
    #[case::while_only_write("while flag:\n    Helper = int\n", "Helper", 100)]
    #[case::write_after_offset("x = 1\n", "x", 0)]
    fn is_defined_before_is_false_without_a_prior_unconditional_write(
        #[case] src: &str,
        #[case] name: &str,
        #[case] offset: u32,
    ) {
        assert!(!analyze(src).is_defined_before(name, TextSize::new(offset)));
    }

    #[rstest]
    #[case::unconditional_after_conditional(
        "if flag:\n    Helper = str\nHelper = int\n",
        "Helper",
        100
    )]
    #[case::unconditional_before_conditional(
        "Helper = str\nif flag:\n    Helper = int\n",
        "Helper",
        100
    )]
    #[case::finally_write_is_unconditional(
        "try:\n    pass\nfinally:\n    Helper = int\n",
        "Helper",
        100
    )]
    #[case::with_body_is_unconditional("with ctx() as _:\n    Helper = int\n", "Helper", 100)]
    #[case::prior_module_write("x = 1\nprint(x)\n", "x", 10)]
    fn is_defined_before_is_true_with_a_prior_unconditional_write(
        #[case] src: &str,
        #[case] name: &str,
        #[case] offset: u32,
    ) {
        assert!(analyze(src).is_defined_before(name, TextSize::new(offset)));
    }

    #[test]
    fn module_function_reads_counts_each_in_module_reference() {
        let analysis = analyze("def f(b, a):\n    pass\n\n\nf(1, 2)\nf(3, 4)\n");
        let reads = analysis.module_function_reads("f").expect("unique def");
        assert_eq!(reads.len(), 2);
    }

    #[test]
    fn module_function_reads_excludes_a_shadowed_local_call() {
        let analysis = analyze("def f(b, a):\n    pass\n\n\ndef g(f):\n    f(1, 2)\n");
        assert!(
            analysis
                .module_function_reads("f")
                .expect("unique def")
                .is_empty(),
            "the call resolves to g's parameter, not module f",
        );
    }

    #[test]
    fn module_function_reads_includes_a_call_before_the_def() {
        let analysis = analyze("def caller():\n    return helper()\n\n\ndef helper():\n    pass\n");
        let reads = analysis
            .module_function_reads("helper")
            .expect("unique def");
        assert_eq!(
            reads.len(),
            1,
            "the forward reference resolves after the walk"
        );
    }

    #[test]
    fn module_function_reads_offset_points_at_the_call_callee() {
        let src = "def f(b, a):\n    pass\n\n\nf(1, 2)\n";
        let analysis = analyze(src);
        let reads = analysis.module_function_reads("f").expect("unique def");
        assert_eq!(reads.len(), 1);
        assert!(src[reads[0].to_usize()..].starts_with("f(1, 2)"));
    }

    #[test]
    fn module_function_reads_orders_a_forward_read_before_a_later_call() {
        let analysis = analyze(
            "def caller():\n    return helper()\n\n\ndef helper():\n    pass\n\n\nhelper()\n",
        );
        let reads = analysis
            .module_function_reads("helper")
            .expect("unique def");
        assert_eq!(reads.len(), 2);
        assert!(
            reads[0] < reads[1],
            "the deferred forward read sorts ahead of the later module-level call",
        );
    }

    #[rstest]
    #[case::star_after_def("def f(b, a):\n    pass\n\n\nfrom x import *\n")]
    #[case::star_before_def("from x import *\n\n\ndef f(b, a):\n    pass\n")]
    #[case::star_in_conditional_overlay(
        "try:\n    from x import *\nexcept ImportError:\n    pass\n\n\ndef f(b, a):\n    pass\n"
    )]
    fn module_function_reads_returns_none_under_a_module_star_import(#[case] src: &str) {
        assert!(analyze(src).module_function_reads("f").is_none());
    }

    #[rstest]
    #[case("def f():\n    pass\n\n\nf = 1\n")]
    #[case("f = lambda: 1\n")]
    #[case("x = 1\n")]
    fn module_function_reads_returns_none_unless_name_is_one_def(#[case] src: &str) {
        assert!(analyze(src).module_function_reads("f").is_none());
    }

    #[test]
    fn module_attribute_count_counts_distinct_attributes() {
        let analysis = analyze("import os\nos.environ\nos.getcwd()\nos.environ\n");
        assert_eq!(analysis.module_attribute_count("os"), 2);
    }

    #[test]
    fn module_attribute_count_records_the_first_segment_of_a_chain() {
        let analysis = analyze("import os\nos.path.join('a', 'b')\n");
        assert_eq!(analysis.module_attribute_count("os"), 1);
    }

    #[rstest]
    #[case("import os\n")]
    #[case("import os\nfoo(os)\n")]
    fn module_attribute_count_is_zero_without_attribute_reads(#[case] src: &str) {
        assert_eq!(analyze(src).module_attribute_count("os"), 0);
    }

    #[test]
    fn module_used_bare_is_false_for_attribute_only_reads() {
        let analysis = analyze("import os\nos.getcwd()\nos.environ\n");
        assert!(!analysis.module_used_bare("os"));
    }

    #[rstest]
    #[case("import os\nfoo(os)\n")]
    #[case("import os\nx = os\n")]
    fn module_used_bare_is_true_for_a_namespace_reference(#[case] src: &str) {
        assert!(analyze(src).module_used_bare("os"));
    }

    #[rstest]
    #[case("X = 1\n")]
    #[case("x = 1\n")]
    fn module_reassigned_is_false_for_write_once_or_unbound(#[case] src: &str) {
        assert!(!analyze(src).module_reassigned("X"));
    }

    #[rstest]
    #[case("X = 1\nX = 2\n")]
    #[case("X = 1\nX += 1\n")]
    #[case("X += 1\n")]
    fn module_reassigned_is_true_when_written_twice_or_augmented(#[case] src: &str) {
        assert!(analyze(src).module_reassigned("X"));
    }

    #[rstest]
    #[case::reused_sibling(
        "head, tail = pair\nuse(tail)\nuse(tail)\nuse(head)\n",
        "head",
        Some(UnpackKind::Exempt)
    )]
    #[case::call_value(
        "name, value = lookup()\nuse(name)\nuse(value)\n",
        "name",
        Some(UnpackKind::Bare)
    )]
    #[case::starred_target(
        "head, *rest = items\nuse(head)\nuse(rest)\n",
        "head",
        Some(UnpackKind::Bare)
    )]
    #[case::nested_unpack(
        "(a, b), c = pair\nuse(a)\nuse(b)\nuse(c)\n",
        "a",
        Some(UnpackKind::Bare)
    )]
    #[case::direct_assignment("x = 1\nuse(x)\n", "x", None)]
    #[case::single_name_unpack("(only,) = pair\nuse(only)\n", "only", None)]
    fn unpack_target_disposition(
        #[case] src: &str,
        #[case] name: &str,
        #[case] expected: Option<UnpackKind>,
    ) {
        let source = parse(src);
        let analysis = source.binding_analysis();
        assert_eq!(
            analysis.unpack_target(module_binding_id(analysis, name)),
            expected
        );
    }

    #[test]
    fn unpack_target_names_the_subscript_for_all_single_use() {
        let source = parse("first, second = batch\nuse(first)\nuse(second)\n");
        let analysis = source.binding_analysis();
        let first = module_binding_id(analysis, "first");
        let second = module_binding_id(analysis, "second");
        assert_matches!(
            analysis.unpack_target(first),
            Some(UnpackKind::Suggested(range, 0)) if &source.text()[range] == "batch"
        );
        assert_matches!(
            analysis.unpack_target(second),
            Some(UnpackKind::Suggested(_, 1))
        );
    }

    #[test]
    fn unpack_target_subscript_handles_an_attribute_value() {
        let source = parse("x, y = box.pair\nuse(x)\nuse(y)\n");
        let analysis = source.binding_analysis();
        let x = module_binding_id(analysis, "x");
        assert_matches!(
            analysis.unpack_target(x),
            Some(UnpackKind::Suggested(range, 0)) if &source.text()[range] == "box.pair"
        );
    }

    #[rstest]
    #[case::if_test("if (n := f()):\n    pass\n", true)]
    #[case::elif_test("if a:\n    pass\nelif (n := f()):\n    pass\n", true)]
    #[case::while_test("while (n := f()):\n    pass\n", true)]
    #[case::assignment_value("x = (n := f())\n", false)]
    #[case::comprehension_guard("ys = [x for x in xs if (n := x)]\n", false)]
    #[case::body_assignment("if a:\n    n = 1\n", false)]
    #[case::if_body("if a:\n    print(n := f())\n", false)]
    fn walrus_in_condition_marks_only_condition_test_walruses(
        #[case] src: &str,
        #[case] expected: bool,
    ) {
        let analysis = analyze(src);
        assert_eq!(
            analysis.walrus_in_condition(module_binding_id(&analysis, "n")),
            expected,
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
            let outer = module_binding_id(&analysis, &name);
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
            let id = module_binding_id(&analysis, &name);
            prop_assert_eq!(analysis.usage_count(id), 1);
        }

        #[test]
        fn unread_name_reports_usage_count_zero(
            tail in "[a-z0-9]{0,5}"
        ) {
            let name = format!("x{tail}");
            let program = format!("{name} = 1\n");
            let analysis = analyze(&program);
            let id = module_binding_id(&analysis, &name);
            prop_assert_eq!(analysis.usage_count(id), 0);
        }
    }
}
