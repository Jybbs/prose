# Primitives

*Prose* is built from a small set of shared primitives that each carry a single responsibility. A rule reads source through [[source]], walks the AST through one of the shared walkers, emits [[edit]] lists, and surfaces diagnostics through the [[pipeline]]. Every rule in the catalog composes from the named pieces below, so a new rule lands as a thin walker plus the per-rule decision rather than a from-scratch implementation. The padding math, the comment-attachment, and the conflict discipline live once and downstream rules consume them.

The graph below traces how a source flows through the primitive set, with each node marking one primitive and each edge marking a consumer relationship *(`A → B` reads as "A is consumed by B")*. The graph nodes match the registries below, and hovering a node previews the primitive's one-line role.

<PrimitivesComposition />

## The Surface

### Public Primitives

Reachable from a downstream Rust consumer in `0.2.x`:

| Primitive | Role |
|---|---|
| [[pipeline]] | Runs the registered rules in deterministic order against a *Source*, reparsing between rules |
| [[rule-id]] | Canonical kebab-case slug identifying each rule across CLI flags, config tables, suppression directives, and diagnostic output |
| [[source]] | Parsed-text wrapper bundling original text, AST, tokens, line index, and the suppression / binding tables. Every rule reads through this value |

### Crate-Internal Primitives

`pub(crate)` in `0.2.x` and stabilizing toward `1.0`, where consumer-implemented rules become reachable:

| Primitive | Role |
|---|---|
| [[aligner]] | Shared alignment math, consumed by [[align-colons]], [[align-comparisons]], [[align-equals]], [[align-imports]], [[align-match-case]] |
| [[binding-analysis]] | Per-*Source* table indexing every write and read of every name in every lexical scope |
| [[cache]] | User-level content-addressed cache, consumed by `prose check` and `prose format` to skip the pipeline on unchanged source |
| [[colon-targets]] | Walker that finds every `:` alignment context, consumed by [[align-colons]] and [[strip-align-padding]] |
| [[docstring]] | PEP 257 docstring walker, consumed by [[docstring-wrap]], [[docstring-frame]], [[docstring-expand]] |
| [[edit]] | The `Edit { range, content }` shape every rule emits and the *Pipeline* applies |
| [[orderer]] | Sibling reorder helper preserving attached comments, consumed by [[alphabetize]] |
| [[suppression-map]] | Per-*Source* index of `# fmt: off` / `# fmt: skip` / `# yapf` / `# prose: ignore[...]` directives |
| [[walker]] | Ignore-aware filesystem walker, consumed by the path-mode CLI |

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
