# Rule Composition

The per-rule pages walk each rule's canonical case in isolation, but the real question for most projects is what happens when several rules apply to the same block. The composition fixtures answer that question. Each case here pairs a small Python source with the rule set it activates, and the before/after pair shows the combined effect of those rules running together in [**Pipeline Order**](/reference/pipeline-order).

The cases are the same `tests/fixtures/composition/` set the binary's integration tests run against, so the rendered output on this page is the canonical answer to what *Prose* does when these rules compose.

## The Canonical Case

One module-level constant carries the full composition story. The right-hand dict starts in a state that puts five rules in motion:

- The literal overflows `code-line-length` on a single line, so [[collection-layout]] breaks it apart.
- Entries arrive in authorship order rather than alphabetical, so [[alphabetize]] sorts them.
- Keys and values both want vertical columns, so [[align-colons]] and [[align-equals]] compute the padding.
- Values use the legacy `Union[…]` form, so [[legacy-union-syntax]] rewrites them to the `|` operator.

The five rules fire in [**Pipeline Order**](/reference/pipeline-order) against the same block, reparsing between each so every rule downstream measures against the rewritten source rather than the original.

<Fixture rule="composition" case="overflow_dict_constants_rewrite_legacy_union" />

## All Cases

<CompositionCards />

## How Composition Resolves

Each case's pipeline runs the listed rules in canonical order, reparsing between rules. A rule downstream of another sees the rewritten source from the upstream rule, not the original source. The cases here cover the common interaction shapes.

### Layout Before Alignment

[[collection-layout]] running upstream of [[align-colons]] commits the per-line shape against which the alignment columns are computed.

### Reorder Before Align

[[alphabetize]] running upstream of [[align-equals]] settles the entry order, meaning the alignment math measures against the final column positions rather than the source ones.

### Docstring Discipline Before Wrap

[[docstring-expand]] and [[docstring-frame]] running upstream of [[docstring-wrap]] settle the quote placement before the body rewrap measures budgets.

### Module Reorder Around a Block Marker

[[alphabetize]]'s module-level branch reorders the assigns above and below a `# fmt: off` block while the bracketed lines stay verbatim. The suppression directive bounds its own scope, so [[alphabetize]] and [[align-equals]] fire freely on every assign outside the bracket and the run boundary respects the marker.

<Fixture rule="composition" case="constants_sort_around_fmt_off" />

For the per-rule canonical case, click any rule chip above. For the deterministic order the pipeline runs in, see the [**Pipeline Order**](/reference/pipeline-order) reference. For the runner that drives the composition, see the [[pipeline]] primitive. For the full rule catalog, see the [**Rules**](/rules/).
