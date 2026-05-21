# BindingAnalysis

<PrimitiveLayout primitive="binding-analysis">

*BindingAnalysis* walks the module once during [[source]] construction and records, for every name introduced or shadowed in a lexical scope, the offsets of every write and read. Several rules read from this table to ask binding-shaped questions, and the single-walk-per-source guarantee is what makes adding new binding-aware rules cheap.

## Public Surface

The *BindingAnalysis* type itself is `pub` and re-exported at the crate root as `prose::BindingAnalysis`, so a downstream consumer can hold a reference to one through [**`Source::binding_analysis`**](/primitives/source). The accessor methods on the type are `pub(crate)` today, so the in-process API is reachable from within the *prose* crate but not from a downstream Rust caller.

A downstream consumer in `0.2.x` can:

- Pass a [[source]] into [**`Pipeline::run`**](/primitives/pipeline) and read diagnostics emitted by binding-aware rules like [[single-use-variables]].
- Observe that the *BindingAnalysis* type exists and is reachable through `source.binding_analysis()`.

A downstream consumer in `0.2.x` cannot:

- Call `assignment_count`, `usage_count`, `binding_kinds`, `binding_name`, `bindings_in_scope`, `first_write_offset`, or `is_defined_before` on the returned reference. All seven readers are `pub(crate)`.
- Implement a custom rule that consumes the binding table. The `Rule` trait is `pub(crate)`.

The methods stabilize toward `1.0`, where every reader becomes `pub` and the `Rule` trait opens so downstream consumers can implement project-specific binding-aware rules.

## Internal Surface

For consumers reading this from within the *prose* crate (*or for readers curious about the surface that will widen at `1.0`*), the table indexes per binding:

- `assignment_count(binding: BindingId) -> usize` counts every write site, including the introducing assignment.
- `usage_count(binding: BindingId) -> usize` counts every read site.
- `binding_kinds(binding: BindingId) -> &[BindingKind]` returns each kind that produced this binding (a single binding may carry several kinds when shadowing or augmented assignment is involved).
- `binding_name(binding: BindingId) -> &str` returns the bound name.
- `bindings_in_scope(stmt: &Stmt) -> impl Iterator<Item = BindingId>` lists every binding introduced in the lexical scope that contains the statement.
- `first_write_offset(binding: BindingId) -> TextSize` returns the offset of the first write.
- `is_defined_before(name: &str, offset: TextSize) -> bool` is the inverse-lookup convenience used by [[unused-future-annotations]] when checking that every name appearing in an annotation resolves to a binding introduced earlier.

The supporting types `BindingId`, `ScopeId`, `BindingKind`, `ScopeKind`, `Binding`, and `Scope` are also `pub(crate)` in `0.2.x`.

## Build Pattern

`BindingAnalysis::new(module: &ModModule)` runs the resolution pass once. The pass walks the AST in source order, tracks every introduction and shadow per lexical scope, and indexes writes and reads by offset. The result is owned by the enclosing [[source]] and handed to consuming rules as `&BindingAnalysis`.

## Re-Using This Primitive

[[single-use-variables]] is the first rule to consume the table, counting writes and reads per binding to surface candidates for inlining. Future rules with binding-shaped questions (*unused imports, shadowing detection, ahead-of-use references, dead-store analysis*) reach for the same primitive without re-walking. The single-walk-per-source guarantee is what makes adding new binding-shaped rules cheap.

A downstream Rust crate consumes *prose* through a Git dependency pinned to a release tag:

```toml
[dependencies]
prose = { git = "https://github.com/Jybbs/prose", tag = "0.2.3" }
```

In `0.2.x` the consumption path is indirect (*through diagnostics emitted by binding-aware rules*) rather than direct method calls. At `1.0` the readers open up.

<template #related>

- [[source]] is the input the analysis builds against, with every binding's offset landing inside the source's text.
- [[single-use-variables]] is the canonical consumer.
- [[pipeline]] drives the rule run that calls into the analysis.
- [[rule-id]] is the handle each rule registers under in the pipeline's ordering.

For the underlying rules catalog, the [**Rules Overview**](/rules/) page walks every shipped rule that may eventually read from the table.

</template>

</PrimitiveLayout>
