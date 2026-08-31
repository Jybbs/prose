---
consumedBy: [alphabetize-siblings, band-constants, bare-imports, inlinable-bindings, miscased-constants, modernize-annotations, prune-inert-imports, reassigned-constants, shed-redundant-base, shed-super-args, simplify-comprehensions]
consumes: [source]
layer: analysis
stability: internal
summary: "Per-source table indexing every write and read of every name in every lexical scope."
tagline: name binding index
---

# BindingAnalysis

<PrimitiveLayout primitive="binding-analysis">

*BindingAnalysis* walks the module once during [[source]] construction and records, for every name introduced or shadowed in a lexical scope, the offsets of every write and read. Several rules read from this table to ask binding-shaped questions, and the single-walk-per-source guarantee is what makes adding new binding-aware rules cheap.

## Public Surface

The *BindingAnalysis* type itself is `pub` and re-exported at the crate root as `prose::BindingAnalysis`, so a downstream consumer can hold a reference to one through [**`Source::binding_analysis`**](/primitives/source). The accessor methods on the type are `pub(crate)` today, so the in-process API is reachable from within the *Prose* crate but not from a downstream Rust caller.

A downstream consumer can:

- Pass a [[source]] into [**`Pipeline::run`**](/primitives/pipeline) and read diagnostics emitted by binding-aware rules like [[inlinable-bindings]].
- Observe that the *BindingAnalysis* type exists and is reachable through `source.binding_analysis()`.

A downstream consumer cannot:

- Call `assignment_count`, `assignment_value_range`, `binding_kinds`, `binding_name`, `bindings_in_scope`, `binds_name`, `first_write_offset`, `is_bound_before`, `is_defined_before`, `is_deleted`, `module_attribute_count`, `module_binding_kinds`, `module_function_reads`, `module_names_read_within`, `module_reads_as_data`, `module_reassigned`, `module_reassigned_without`, `module_usage_count`, `module_used_bare`, `read_offsets`, `scope_binds`, or `unpack_target` on the returned reference. Every reader is `pub(crate)`.
- Implement a custom rule that consumes the binding table. The `Rule` trait is `pub(crate)`.

The methods stabilize toward `1.0`, where every reader becomes `pub` and the `Rule` trait opens so downstream consumers can implement project-specific binding-aware rules.

## Internal Surface

For consumers reading this from within the *Prose* crate (*or for readers curious about the surface that will widen at `1.0`*), the table indexes per binding:

- `assignment_count(binding: BindingId) -> usize` counts every write site, including the introducing assignment.
- `assignment_value_range(offset: TextSize) -> Option<TextRange>` returns the source range of the value bound at a direct `name = value` or `name: T = value` write, which [[inlinable-bindings]] reads to name the inline candidate, and `None` for a tuple or list target.
- `binding_kinds(binding: BindingId) -> &[BindingKind]` returns each kind that produced this binding *(a single binding may carry several kinds when shadowing or augmented assignment is involved)*.
- `binding_name(binding: BindingId) -> &str` returns the bound name.
- `bindings_in_scope(stmt: &Stmt) -> impl Iterator<Item = BindingId>` lists every binding introduced in the lexical scope that contains the statement.
- `binds_name(name: &str) -> bool` reports whether any scope in the module binds a name, which [[simplify-comprehensions]] reads to hold every call to a constructor the module rebinds, and which [[shed-super-args]] reads to leave every call in place where the module binds `super` or `__class__` itself.
- `first_write_offset(binding: BindingId) -> TextSize` returns the offset of the first write.
- `is_bound_before(name: &str, offset: TextSize) -> bool` reports whether a module-scope write of a name sits before an offset, a write nested in a conditional branch included where `is_defined_before` counts the unconditional writes alone, which [[band-constants]] reads to pin a constant whose value names a definition below it that would rebind an earlier write.
- `is_defined_before(name: &str, offset: TextSize) -> bool` is the inverse-lookup convenience used by [[prune-inert-imports]] when checking that every name appearing in an annotation resolves to an unconditional binding introduced earlier *(a name written only inside a conditional branch like `if`, `for`, `while`, `try`, or `match` reads as runtime-unavailable)*, and read by [[shed-redundant-base]] to hold a header whose `object` base a module-scope write rebound ahead of the class.
- `is_deleted(name: &str) -> bool` reports whether a `del` statement anywhere in the module names a binding, which [[prune-inert-imports]] reads to hold an import whose `del` would otherwise be left raising `NameError`, and [[inlinable-bindings]] reads to hold a binding whose inline would strand one.
- `module_attribute_count(name: &str) -> usize` counts the distinct attributes read off a module-scope name *(`os.environ` and `os.getcwd` count as two)*, which [[bare-imports]] reads to weigh how widely a bare import reaches.
- `module_binding_kinds(name: &str) -> &[BindingKind]` returns the write kinds recorded against a module-scope name, empty where the name is unbound there.
- `module_function_reads(name: &str) -> Option<&[TextSize]>` returns the read offsets of a module-scope name bound exactly once as a function definition, which [[reflow-calls]] uses through `module_call_params` to resolve the signature a module-function call binds, so it names the call's positional arguments when exploding it.
- `module_names_read_within(ranges: &[TextRange]) -> Vec<FxHashSet<&str>>` names the module-scope bindings read inside each of a set of ascending, non-overlapping ranges, one set per range, which [[alphabetize-siblings]] reads through `call_reachable` to widen a definition's reach along the call graph before a sort moves it past a statement that runs it.
- `module_reads_as_data(name: &str) -> bool` reports whether a module-scope name is read only where data stands and never where a type stands, one read in an annotation outranking every data read.
- `module_reassigned(name: &str) -> bool` reports whether a module-scope name carries more than one write or an augmented assignment, which [[reassigned-constants]], [[miscased-constants]], and [[alphabetize-siblings]] read to skip names that are not write-once.
- `module_reassigned_without(name: &str, dropped: impl Fn(TextSize) -> bool) -> bool` answers what `module_reassigned` answers once every write `dropped` names is removed, which [[prune-inert-imports]] reads so a repeat it is already dropping stops counting as the rebind that would otherwise hold the first binding.
- `module_usage_count(name: &str) -> usize` counts every read recorded against a module-scope name, which [[modernize-annotations]] weighs against the reads its own rewrite consumed and [[prune-inert-imports]] reads directly to decide whether an import binding still has a reader.
- `module_used_bare(name: &str) -> bool` reports whether a module-scope name is ever read without an attribute access *(the namespace object itself is used)*, which [[bare-imports]] reads before suggesting a `from` import.
- `read_offsets(binding: BindingId) -> &[TextSize]` returns every offset at which a binding is read, ascending, which [[inlinable-bindings]] reads to locate the single read it measures the inline against. A walrus target counts its own value as one of them.
- `scope_binds(stmt: &Stmt, name: &str) -> bool` reports whether the local scope of a `def` binds a name, which [[shed-super-args]] reads to hold a call whose first argument names a local rather than the enclosing class.
- `unpack_target(binding: BindingId) -> Option<UnpackKind>` returns the unpack disposition of a binding whose sole write is a multi-name tuple or list target, which [[inlinable-bindings]] reads to choose between naming a subscript rewrite and withholding the finding.

The supporting types `BindingId`, `ScopeId`, `BindingKind`, `ScopeKind`, `UnpackKind`, `Binding`, and `Scope` are also `pub(crate)` today. `BindingKind` enumerates the categories of write event the table records: `Assignment`, `AugAssign`, `ClassDef`, `Comprehension`, `ExceptHandler`, `For`, `FunctionDef`, `Import`, `Parameter`, `Walrus`, `With`. `ScopeKind` covers `Class`, `Comprehension`, `Function`, `Module`, matching Python's lexical-scope categories. `UnpackKind` covers `Suggested` and `Unresolved`, the dispositions `unpack_target` reports for a multi-name unpack target.

## Build Pattern

`BindingAnalysis::new(module: &ModModule)` runs the resolution pass once during [[source]] construction. The pass walks the AST in source order, tracks every introduction and shadow per lexical scope, and indexes writes and reads by offset. The result is owned by the enclosing [[source]] and handed to consuming rules as `&BindingAnalysis`.

A fresh analysis is built each time [[source]] is constructed or reparsed, so the offsets a rule reads always match the *Source* it's running against. Inside one rule's `apply` the table is immutable, and across rules the pipeline reparses, which rebuilds the analysis against the new text, so a rule that depends on a previous rule's edits sees a fresh table reflecting the rewritten source.

## Re-Using This Primitive

[[inlinable-bindings]] is the first rule to consume the table, counting writes and reads per binding to surface candidates for inlining. Future rules with binding-shaped questions (*unused imports, shadowing detection, ahead-of-use references, dead-store analysis*) reach for the same primitive without re-walking. The single-walk-per-source guarantee is what makes adding new binding-shaped rules cheap.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page. The consumption path runs indirectly through diagnostics emitted by binding-aware rules rather than through direct method calls, and at `1.0` the readers open up so a downstream rule can query the table itself.

<template #related>

- [[source]] is the input the analysis builds against, with every binding's offset landing inside the source's text.
- [[inlinable-bindings]] is the canonical consumer.
- [[edit]] is the output shape binding-aware rules emit, with each edit's range named against an offset the analysis indexes.
- [[pipeline]] drives the rule run that calls into the analysis.
- [[rule-id]] is the handle each rule registers under in the pipeline's ordering.

For the underlying rules catalog, the [**Rules**](/rules/) page walks every shipped rule across categories, including the binding-aware rules that read from this table.

</template>

</PrimitiveLayout>
