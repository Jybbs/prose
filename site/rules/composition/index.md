# Rule Composition

The per-rule pages walk each rule's canonical case in isolation, but the real question for most projects is what happens when several rules apply to the same block. The composition fixtures answer that question. Each case here pairs a small Python source with the `[tool.prose.rules.<rule>]` configuration that names the active rule set, and the before/after pair shows the combined effect of those rules running together in [**Pipeline Order**](/reference/pipeline-order).

The cases are the same `tests/fixtures/composition/` set the binary's integration tests run against, so the rendered output on this page is the canonical answer to what *Prose* does when these rules compose.

<CompositionGrid />

## How Composition Resolves

Each case's pipeline runs the listed rules in canonical order, reparsing between rules. A rule downstream of another sees the rewritten source from the upstream rule, not the original source. The cases here cover the common interaction shapes.

### Layout Before Alignment

[[collection-layout]] running upstream of [[align-colons]] commits the per-line shape against which the alignment columns are computed.

### Reorder Before Align

[[alphabetize]] running upstream of [[align-equals]] settles the entry order, meaning the alignment math measures against the final column positions rather than the source ones.

### Docstring Discipline Before Wrap

[[no-single-line-docstrings]] and [[multi-line-docstrings]] running upstream of [[docstring-wrap]] settle the quote placement before the body rewrap measures budgets.

For the per-rule canonical case, click any rule chip above. For the deterministic order the pipeline runs in, see the [**Pipeline Order**](/reference/pipeline-order) reference. For the runner that drives the composition, see the [[pipeline]] primitive. For the full rule catalog, see the [**Rules Overview**](/rules/).
