---
description: "Small sources run through several rules together, in pipeline order."
---

# Rule Composition

Each case on this page pairs a small Python source with the rule set it turns on and shows what those rules write when they run together in [**Pipeline Order**](/reference/pipeline-order). A rule page shows the cases its own rule takes part in, whereas this page runs each case through the whole listed set. The cards below render the previewable cases from `crate/tests/fixtures/composition/`, and the binary's integration tests run every case in that directory.

## The Canonical Case

One module-level constant exercises a rule from each family:

- Its values use the legacy `Union[…]` form, so [[modernize-annotations]] rewrites them to the `|` operator and removes the `typing` import they read through.
- The literal overflows `code-line-length` on one line, so [[reflow-collections]] writes it one entry per line.
- The entries sit in the order the author wrote them rather than alphabetical, so [[alphabetize-siblings]] sorts them.
- The keys then sit one per line in a column, so [[align-colons]] computes the padding from the widths the rewrite leaves.
- The removed import leaves a gap where it stood, so [[space-statements]] closes it.

The rules run against the same block, re-parsing between each so every later rule measures the rewritten source.

<Fixture rule="composition" case="overflow_dict_constants_modernize_unions" />

## The Cases

<CompositionCards />

## How Composition Resolves

Each case runs its listed rules in canonical order, and the sections below cover the common interactions.

### Layout Before Alignment

[[reflow-collections]] runs before [[align-colons]] and writes the one-entry-per-line layout the alignment columns are computed from.

### Reorder Before Align

[[alphabetize-siblings]] runs before [[align-equals]] and settles the entry order, so the alignment math measures the final column positions rather than the source ones.

### Docstring Discipline Before Wrap

[[expand-docstrings]] and [[frame-docstrings]] run before [[wrap-docstrings]] and settle the quote placement before the body rewrap measures its budget.

### Module Reorder Around a Block Marker

[[band-constants]] sorts the constants above a `# fmt: off` block while the lines inside the block stay as written, so both it and [[align-equals]] act freely outside the marked region.

<Fixture rule="composition" case="constants_sort_around_fmt_off" />

Click any rule chip above for its canonical case. The [**Pipeline Order**](/reference/pipeline-order) reference lists the order the pipeline runs in, the [[pipeline]] primitive covers the runner, and the [**Rules**](/rules/) catalog lists the rest.
