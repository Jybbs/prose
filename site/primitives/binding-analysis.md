---
consumedBy: [alphabetize-siblings, band-constants, bare-imports, inlinable-bindings, miscased-constants, modernize-annotations, prune-inert-imports, reassigned-constants, shed-redundant-base, shed-super-args, simplify-comprehensions]
consumes: [source]
layer: analysis
stability: internal
summary: "Per-source table recording every write and read of every name in every lexical scope."
tagline: name binding index
---

# BindingAnalysis

<PrimitiveLayout primitive="binding-analysis">

*BindingAnalysis* reads the module once, on the first call to `binding_analysis()` against a [[source]], and records the offsets of every write and read of every name introduced or shadowed in a lexical scope. Several rules read this table for their binding questions, and because the table is built once per source, adding a new binding-aware rule costs no extra traversal.

## Public Surface

The *BindingAnalysis* type itself is `pub` and re-exported at the crate root as `prose::BindingAnalysis`, so a downstream consumer can reach one through [**`Source::binding_analysis`**](/primitives/source). The reader methods on the type are `pub(crate)` today, so code inside the *Prose* crate can call them and a downstream Rust caller cannot.

A downstream consumer can:

- Pass a [[source]] into [**`Pipeline::run`**](/primitives/pipeline) and read the diagnostics binding-aware rules like [[inlinable-bindings]] emit.
- Reach the *BindingAnalysis* value through `source.binding_analysis()`.

A downstream consumer cannot:

- Call `assignment_count`, `assignment_value_range`, `binding_kinds`, `binding_name`, `bindings_in_scope`, `binds_name`, `first_unconditional_write`, `first_write_offset`, `is_bound_before`, `is_defined_before`, `is_deleted`, `module_attribute_count`, `module_binding_kinds`, `module_function_reads`, `module_names_read_within`, `module_reads_as_data`, `module_reassigned`, `module_reassigned_without`, `module_usage_count`, `module_used_bare`, `read_offsets`, `scope_binds`, or `unpack_target` on the returned reference. Every reader is `pub(crate)`.
- Implement a custom rule that reads the binding table. The `Rule` trait is `pub(crate)`.

The methods open toward `1.0`, when every reader becomes `pub` and the `Rule` trait opens so a downstream consumer can implement a project-specific binding-aware rule.

## Internal Surface

For code inside the *Prose* crate (*and for a reader curious about the API that widens at `1.0`*), the table records per binding:

- `assignment_count(binding: BindingId) -> usize` counts every write site, the introducing assignment included.
- `assignment_value_range(offset: TextSize) -> Option<TextRange>` returns the source range of the value bound at a direct `name = value` or `name: T = value` write, or `None` for a tuple or list target. [[inlinable-bindings]] reads it to name the inline candidate.
- `binding_kinds(binding: BindingId) -> &[BindingKind]` returns each kind of write that produced this binding *(one binding may carry several kinds where shadowing or augmented assignment is involved)*.
- `binding_name(binding: BindingId) -> &str` returns the bound name.
- `bindings_in_scope(stmt: &Stmt) -> impl Iterator<Item = BindingId>` lists every binding declared directly inside the local scope of the `def` at `stmt`, and yields nothing for any other statement.
- `binds_name(name: &str) -> bool` reports whether any scope in the module binds a name, which [[simplify-comprehensions]] reads to leave every call to a constructor the module rebinds as written, and which [[shed-super-args]] reads to leave every call as written where the module binds `super` or `__class__` itself.
- `first_unconditional_write(name: &str) -> Option<TextSize>` returns the offset of the earliest unconditional module-scope write of a name, and `None` where every write sits inside a conditional branch or the name is unbound at module scope. [[prune-inert-imports]] reads it to check that every name in an annotation resolves to an unconditional binding written ahead of it *(a name written only inside an `if`, `for`, `while`, `try`, or `match` branch reads as unavailable at runtime)*.
- `first_write_offset(binding: BindingId) -> TextSize` returns the offset of the first write.
- `is_bound_before(name: &str, offset: TextSize) -> bool` reports whether a module-scope write of a name sits before an offset, a write nested in a conditional branch included where `is_defined_before` counts the unconditional writes alone. [[band-constants]] reads it to keep a constant in place when its value names a definition below it that would rebind an earlier write.
- `is_defined_before(name: &str, offset: TextSize) -> bool` reports whether an unconditional module-scope write of a name sits before an offset, which [[shed-redundant-base]] reads to leave a class header as written when a module-scope write rebound its `object` base ahead of the class.
- `is_deleted(name: &str) -> bool` reports whether a `del` statement anywhere in the module names a binding, which [[prune-inert-imports]] reads to keep an import whose removal would leave its `del` raising `NameError`, and [[inlinable-bindings]] reads to keep a binding whose inlining would strand one.
- `module_attribute_count(name: &str) -> usize` counts the distinct attributes read off a module-scope name *(`os.environ` and `os.getcwd` count as two)*, which [[bare-imports]] reads to measure how widely a bare import is used.
- `module_binding_kinds(name: &str) -> &[BindingKind]` returns the write kinds recorded against a module-scope name, empty where the name is unbound there.
- `module_function_reads(name: &str) -> Option<&[TextSize]>` returns the read offsets of a module-scope name bound exactly once as a function definition, which [[reflow-calls]] reads through `module_call_params` to resolve the signature a call to a module function binds, so it can name the call's positional arguments when it explodes the call.
- `module_names_read_within(ranges: &[TextRange]) -> Vec<FxHashSet<&str>>` names the module-scope bindings read inside each of a set of ascending, non-overlapping ranges, one set per range, which [[alphabetize-siblings]] reads through `call_reachable` to extend a definition's reach along the call graph before a sort moves it past a statement that runs it.
- `module_reads_as_data(name: &str) -> bool` reports whether a module-scope name is read only where data stands and never where a type stands, one read in an annotation outranking every data read.
- `module_reassigned(name: &str) -> bool` reports whether a module-scope name carries more than one write or an augmented assignment, which [[reassigned-constants]], [[miscased-constants]], and [[band-constants]] read to skip names that are not write-once.
- `module_reassigned_without(name: &str, dropped: impl Fn(TextSize) -> bool) -> bool` answers what `module_reassigned` answers once every write `dropped` names is removed, which [[prune-inert-imports]] reads so a repeated import it is already removing stops counting as the rebind that would otherwise keep the first binding.
- `module_usage_count(name: &str) -> usize` counts every read recorded against a module-scope name, which [[modernize-annotations]] weighs against the reads its own rewrite consumed and [[prune-inert-imports]] reads directly to test whether an import binding still has a reader.
- `module_used_bare(name: &str) -> bool` reports whether a module-scope name is ever read without an attribute access *(the namespace object itself is used)*, which [[bare-imports]] reads before suggesting a `from` import.
- `read_offsets(binding: BindingId) -> &[TextSize]` returns every offset at which a binding is read, ascending, which [[inlinable-bindings]] reads to locate the single read it measures the inline against. A walrus target counts its own value as one of them.
- `scope_binds(stmt: &Stmt, name: &str) -> bool` reports whether the local scope of a `def` binds a name, which [[shed-super-args]] reads to leave a call as written when its first argument names a local rather than the enclosing class.
- `unpack_target(binding: BindingId) -> Option<UnpackKind>` returns the unpack disposition of a binding whose sole write is a multi-name tuple or list target, which [[inlinable-bindings]] reads to either name a subscript rewrite or withhold the finding.

`BindingId`, `BindingKind`, and `UnpackKind` are `pub(crate)` today, whereas `ScopeId`, `ScopeKind`, `Binding`, and `Scope` are private to the module. `BindingKind` enumerates the categories of write event the table records: `Assignment`, `AugAssign`, `ClassDef`, `Comprehension`, `ExceptHandler`, `For`, `FunctionDef`, `Import`, `Parameter`, `Walrus`, `With`. `ScopeKind` covers `Class`, `Comprehension`, `Function`, `Module`, matching Python's lexical-scope categories. `UnpackKind` covers `Suggested` and `Unresolved`, the dispositions `unpack_target` reports for a multi-name unpack target.

## Build Pattern

`BindingAnalysis::new(module: &ModModule)` runs the resolution pass on the first read of `binding_analysis()`, so a run whose rules never read it never pays for the traversal. The pass reads the AST in source order, tracks every introduction and shadow per lexical scope, and records writes and reads by offset. The enclosing [[source]] owns the result and hands it to a consuming rule as `&BindingAnalysis`.

Across a reparse the table travels with the *Source* rather than being rebuilt, every offset it records moved through the `SourceMap` of the applied edits, so the offsets a rule reads always match the text it runs against. After a rule whose edits change a binding, or an edit that replaced a token the table names, the next read rebuilds the table against the new text. Inside one rule's `apply` the table is immutable.

## Re-Using This Primitive

[[inlinable-bindings]] is the canonical consumer, counting writes and reads per binding to find candidates for inlining. A rule with a binding question of its own *(an unused import, a shadowed name, a reference ahead of its definition, a dead store)* reads the same table without traversing the module again. The traversal runs once per source and again only after a rule whose edits change a binding, which is what keeps a new binding-aware rule cheap.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page, and a downstream consumer that depends on the crate reaches the table only through the diagnostics binding-aware rules emit, not through direct method calls. At `1.0` the readers open so a downstream rule can query the table itself.

<template #related>

- [[source]] is the input the analysis builds against, with every binding's offset pointing into the source's text.
- [[inlinable-bindings]] is the canonical consumer.
- [[edit]] is the output type binding-aware rules emit, with each edit's range named against an offset the analysis records.
- [[pipeline]] drives the rule run that reads the analysis.
- [[rule-id]] is the handle each rule registers under in the pipeline's ordering.

For the underlying rules catalog, the [**Rules**](/rules/) page lists every shipped rule across categories, the binding-aware rules that read this table included.

</template>

</PrimitiveLayout>
