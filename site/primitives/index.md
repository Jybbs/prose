# Primitives

*Prose* is built from a small set of shared primitives that each carry a single responsibility. A rule reads source through [[source]], walks the AST through one of the shared walkers, emits [[edit]] lists, and surfaces diagnostics through the [[pipeline]]. Every rule in the catalog is built from these eleven named pieces, meaning a new rule lands as a thin walker plus the per-rule decision rather than a from-scratch implementation. The reuse economy is what makes adding a rule cheap, wherein the math, the comment-attachment, and the conflict discipline live once and downstream rules consume them.

## The Surface

**Public primitives** *(reachable from a downstream Rust consumer in `0.2.x`)*

- [[source]] · parsed-text wrapper bundling original text, AST, tokens, line index, and the suppression / binding tables. Every rule reads through this value
- [[pipeline]] · runs the registered rules in deterministic order against a *Source*, reparsing between rules
- [[rule-id]] · canonical kebab-case slug identifying each rule across CLI flags, config tables, suppression directives, and diagnostic output

**Crate-internal primitives** *(`pub(crate)` in `0.2.x`, stabilizing toward `1.0`)*

- [[binding-analysis]] · per-*Source* table indexing every write and read of every name in every lexical scope
- [[suppression-map]] · per-*Source* index of `# fmt: off` / `# fmt: skip` / `# yapf` / `# prose: ignore[...]` directives
- [[aligner]] · shared alignment math, consumed by [[align-equals]], [[align-colons]], [[align-imports]], [[match-case-align]]
- [[orderer]] · sibling reorder helper preserving attached comments, consumed by [[alphabetize]]
- [[colon-targets]] · walker that finds every `:` alignment context, consumed by [[align-colons]] and [[singleton-rule]]
- [[edit]] · the `Edit { range, content }` shape every rule emits and the *Pipeline* applies
- [[docstring]] · PEP 257 docstring walker, consumed by [[docstring-wrap]], [[multi-line-docstrings]], [[no-single-line-docstrings]]
- [[walker]] · ignore-aware filesystem walker, consumed by the path-mode CLI

## Reading Order

For a downstream Rust consumer integrating *Prose* through the public surface, the load-bearing reads are [[source]] *(input)*, [[pipeline]] *(runner)*, and [[rule-id]] *(slug type)*. The other pages cover the internal machinery, useful when adding a rule or auditing the pipeline's flow.

The [**Rules Overview**](/rules/) walks every rule each primitive shows up under, the [**Configuration**](/reference/configuration) reference covers the `[tool.prose]` table that drives the *Pipeline*'s rule selection, and the [**Pipeline Order**](/reference/pipeline-order) reference covers the deterministic order rules fire in.
