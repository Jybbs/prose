# Primitives

*Prose* is built from a small set of shared primitives that each carry a single responsibility. A rule reads source through [[source]], walks the AST through one of the shared walkers, emits [[edit]] lists, and surfaces diagnostics through the [[pipeline]]. Every rule in the catalog composes from the named pieces below, so a new rule lands as a thin walker plus the per-rule decision rather than a from-scratch implementation. The padding math, the comment-attachment, and the conflict discipline live once and downstream rules consume them.

The graph below traces how a source flows through the primitive set, with each node marking one primitive and each edge marking a consumer relationship *(`A → B` reads as "A is consumed by B")*. The graph nodes match the registries below, and hovering a node previews the primitive's one-line role.

<PrimitivesComposition />

## The Surface

### Public Primitives

Reachable from a downstream Rust consumer today:

<PrimitiveSurface stability="public" />

### Crate-Internal Primitives

`pub(crate)` today and stabilizing toward `1.0`, where consumer-implemented rules become reachable:

<PrimitiveSurface stability="internal" />

## Reading Order

For a downstream Rust consumer integrating *Prose* through the public surface, the load-bearing reads are [[source]] *(input)*, [[pipeline]] *(runner)*, and [[rule-id]] *(slug type)*. The three together cover construction, execution, and the slug shape that flows through every CLI flag and config table.

For a rule author working inside the *Prose* crate, the reading path starts at [[edit]] *(the unit every rule emits)* and walks through [[pipeline]] *(the runner the rule registers with)*. From there, the right walker primitive depends on what the rule does:

- [[aligner]] for rules that pad to a column.
- [[orderer]] for rules that reorder siblings.
- [[colon-targets]] for rules that align around `:` contexts.
- [[docstring]] for rules over PEP 257 docstrings.
- [[binding-analysis]] for rules that ask binding-shaped questions.

[[source]] is the input every walker reads against, and [[suppression-map]] is the filter every emission passes through.

The [**Rules**](/rules/) page walks every rule each primitive shows up under, the [**Configuration**](/reference/configuration) reference covers the `[tool.prose]` table that drives the *Pipeline*'s rule selection, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the deterministic order rules fire in.
