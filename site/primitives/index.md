---
description: "The shared Rust primitives every rule is composed from, and which a downstream crate links against."
---

# Primitives

*Prose* is built from a small set of shared primitives, each with one responsibility. A rule reads source through [[source]], reads the AST through one of the shared walkers, emits [[edit]] lists, and reports diagnostics through the [[pipeline]]. Every rule in the catalog is composed from the pieces named below, so a new rule is a thin walker plus its own per-rule decision rather than an implementation from scratch. The padding math, the comment attachment, and the conflict discipline each live in one place, and the rules read them from there.

The graph below traces how a source flows through the primitives, each node one primitive and each edge one consumer relationship *(`A → B` reads as "A is consumed by B")*. The nodes match the registries below, and hovering a node shows the primitive's one-line role.

<PrimitivesComposition />

## The Surface

### Public Primitives

Reachable from a downstream Rust consumer today:

<PrimitiveSurface stability="public" />

### Crate-Internal Primitives

`pub(crate)` today and opening toward `1.0`, when consumer-implemented rules become reachable:

<PrimitiveSurface stability="internal" />

## Reading Order

For a downstream Rust consumer integrating *Prose* through the public API, the pages to read are [[source]] *(input)*, [[pipeline]] *(runner)*, and [[rule-id]] *(slug type)*. The three together cover construction, execution, and the slug type every CLI flag and config table names a rule by.

For a rule author working inside the *Prose* crate, the reading path starts at [[edit]] *(the unit every rule emits)* and continues to [[pipeline]] *(the runner the rule registers with)*. From there, which walker primitive to read depends on what the rule does:

- [[aligner]] for a rule that pads to a column.
- [[orderer]] for a rule that reorders siblings.
- [[colon-targets]] for a rule that aligns around a `:`.
- [[docstring]] for a rule over PEP 257 docstrings.
- [[binding-analysis]] for a rule that reads name bindings.

[[source]] is the input every walker reads, and [[suppression-map]] is the filter every emitted edit and diagnostic passes through.

The [**Rules**](/rules/) page lists every rule each primitive appears under, the [**Configuration**](/reference/configuration) reference covers the `[tool.prose]` table that drives the *Pipeline*'s rule selection, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the fixed order the rules run in.
