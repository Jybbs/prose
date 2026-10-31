---
description: "Small sources run through several rules together, in pipeline order."
---

# Rule Composition

Each case on this page pairs a small Python source with the rule set it turns on and shows what those rules write when they run together in [**Pipeline Order**](/reference/pipeline-order), which the [[pipeline]] runner applies. A rule page shows the cases its own rule takes part in, whereas this page runs each case through the whole listed set. The cards below render the previewable cases from `crate/tests/fixtures/composition/`, and the binary's integration tests run every case in that directory.

## The Canonical Case

One module-level constant exercises a rule from each of the alignment, formatting, layout, and ordering families:

- Its values use the legacy `Union[…]` form, so [[modernize-annotations]] rewrites them to the `|` operator and removes the `typing` import they read through, together with the blank line below it.
- The literal overflows `code-line-length` on one line, so [[reflow-collections]] writes it one entry per line.
- The entries sit in the order the author wrote them rather than alphabetical, so [[alphabetize-siblings]] sorts them.
- The keys then sit one per line in a column, so [[align-colons]] computes the padding from the widths the rewrite leaves.
- [[space-statements]] runs too and writes nothing, since the import leaves no gap where it stood.

<Fixture rule="composition" case="overflow_dict_constants_modernize_unions" />

## The Cases

<CompositionCards />

## Module Reorder Around a Block Marker

[[band-constants]] sorts the constants above a `# fmt: off` block while the lines inside the block stay as written, so both it and [[align-equals]] act freely outside the marked region.

<Fixture rule="composition" case="constants_sort_around_fmt_off" />
