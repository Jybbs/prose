---
stability: internal
---

# BindingAnalysis

<PrimitiveLayout primitive="binding-analysis">

*BindingAnalysis* walks the module once during [[source]] construction and records, for every name introduced or shadowed in a lexical scope, the offsets of every write and read. Several rules read from this table to ask binding-shaped questions, and the single-walk-per-source guarantee is what makes adding new binding-aware rules cheap.

## Public Surface

The *BindingAnalysis* type itself is `pub` and re-exported at the crate root as `prose::BindingAnalysis`, so a downstream consumer can hold a reference to one through [**`Source::binding_analysis`**](/primitives/source). The accessor methods on the type are `pub(crate)` today, so the in-process API is reachable from within the *Prose* crate but not from a downstream Rust caller.

A downstream consumer in `0.2.x` can:

- Pass a [[source]] into [**`Pipeline::run`**](/primitives/pipeline) and read diagnostics emitted by binding-aware rules like [[single-use-variables]].
- Observe that the *BindingAnalysis* type exists and is reachable through `source.binding_analysis()`.

A downstream consumer in `0.2.x` cannot:

- Call `assignment_count`, `assignment_value_range`, `binding_kinds`, `binding_name`, `bindings_in_scope`, `first_write_offset`, `is_defined_before`, `module_attribute_count`, `module_function_reads`, `module_reassigned`, `module_used_bare`, `unpack_target`, `usage_count`, or `walrus_in_condition` on the returned reference. Every reader is `pub(crate)`.
- Implement a custom rule that consumes the binding table. The `Rule` trait is `pub(crate)`.

The methods stabilize toward `1.0`, where every reader becomes `pub` and the `Rule` trait opens so downstream consumers can implement project-specific binding-aware rules.

## Internal Surface

For consumers reading this from within the *Prose* crate (*or for readers curious about the surface that will widen at `1.0`*), the table indexes per binding:

- `assignment_count(binding: BindingId) -> usize` counts every write site, including the introducing assignment.
- `assignment_value_range(offset: TextSize) -> Option<TextRange>` returns the source range of the value bound at a direct `name = value` or `name: T = value` write, which [[single-use-variables]] reads to name the inline candidate, and `None` for a tuple or list target.
- `binding_kinds(binding: BindingId) -> &[BindingKind]` returns each kind that produced this binding *(a single binding may carry several kinds when shadowing or augmented assignment is involved)*.
- `binding_name(binding: BindingId) -> &str` returns the bound name.
- `bindings_in_scope(stmt: &Stmt) -> impl Iterator<Item = BindingId>` lists every binding introduced in the lexical scope that contains the statement.
- `first_write_offset(binding: BindingId) -> TextSize` returns the offset of the first write.
- `is_defined_before(name: &str, offset: TextSize) -> bool` is the inverse-lookup convenience used by [[unused-future-annotations]] when checking that every name appearing in an annotation resolves to an unconditional binding introduced earlier *(a name written only inside a conditional branch like `if`, `for`, `while`, `try`, or `match` reads as runtime-unavailable)*.
- `module_attribute_count(name: &str) -> usize` counts the distinct attributes read off a module-scope name *(`os.environ` and `os.getcwd` count as two)*, which [[bare-imports]] reads to weigh how widely a bare import reaches.
- `module_function_reads(name: &str) -> Option<&[TextSize]>` returns the read offsets of a module-scope name bound exactly once as a function definition, which [[call-layout]] uses through `module_call_params` to resolve the signature a module-function call binds, so it names the call's positional arguments when exploding it.
- `module_reassigned(name: &str) -> bool` reports whether a module-scope name carries more than one write or an augmented assignment, which [[reassigned-constants]] and [[alphabetize]] read to skip names that are not write-once.
- `module_used_bare(name: &str) -> bool` reports whether a module-scope name is ever read without an attribute access *(the namespace object itself is used)*, which [[bare-imports]] reads before suggesting a `from` import.
- `unpack_target(binding: BindingId) -> Option<UnpackKind>` returns the unpack disposition of a binding whose sole write is a multi-name tuple or list target, which [[single-use-variables]] reads to choose between exempting the target and naming a subscript rewrite.
- `usage_count(binding: BindingId) -> usize` counts every read site.
- `walrus_in_condition(binding: BindingId) -> bool` reports whether a binding's walrus write lands in the test of an `if`, `elif`, or `while`, which [[single-use-variables]] reads to exempt that assign-and-test walrus from the lint.

The supporting types `BindingId`, `ScopeId`, `BindingKind`, `ScopeKind`, `UnpackKind`, `Binding`, and `Scope` are also `pub(crate)` in `0.2.x`. `BindingKind` enumerates the categories of write event the table records: `Assignment`, `AugAssign`, `ClassDef`, `Comprehension`, `ExceptHandler`, `For`, `FunctionDef`, `Import`, `Parameter`, `Walrus`, `With`. `ScopeKind` covers `Class`, `Comprehension`, `Function`, `Module`, matching Python's lexical-scope categories. `UnpackKind` covers `Bare`, `Exempt`, and `Suggested`, the dispositions `unpack_target` reports for a multi-name unpack target.

## Build Pattern

`BindingAnalysis::new(module: &ModModule)` runs the resolution pass once during [[source]] construction. The pass walks the AST in source order, tracks every introduction and shadow per lexical scope, and indexes writes and reads by offset. The result is owned by the enclosing [[source]] and handed to consuming rules as `&BindingAnalysis`.

A fresh analysis is built each time [[source]] is constructed or reparsed, so the offsets a rule reads always match the *Source* it's running against. Inside one rule's `apply` the table is immutable, and across rules the pipeline reparses, which rebuilds the analysis against the new text, so a rule that depends on a previous rule's edits sees a fresh table reflecting the rewritten source.

## Re-Using This Primitive

[[single-use-variables]] is the first rule to consume the table, counting writes and reads per binding to surface candidates for inlining. Future rules with binding-shaped questions (*unused imports, shadowing detection, ahead-of-use references, dead-store analysis*) reach for the same primitive without re-walking. The single-walk-per-source guarantee is what makes adding new binding-shaped rules cheap.

The Cargo dependency line *(`prose = { git = "...", tag = "<version>" }`)* lives on the [[source]] page. In `0.2.x` the consumption path runs indirectly through diagnostics emitted by binding-aware rules rather than through direct method calls, and at `1.0` the readers open up so a downstream rule can query the table itself.

<template #related>

- [[source]] is the input the analysis builds against, with every binding's offset landing inside the source's text.
- [[single-use-variables]] is the canonical consumer.
- [[edit]] is the output shape binding-aware rules emit, with each edit's range named against an offset the analysis indexes.
- [[pipeline]] drives the rule run that calls into the analysis.
- [[rule-id]] is the handle each rule registers under in the pipeline's ordering.

For the underlying rules catalog, the [**Rules**](/rules/) page walks every shipped rule across categories, including the binding-aware rules that read from this table.

</template>

</PrimitiveLayout>
