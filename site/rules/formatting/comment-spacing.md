---
caption : "Settles every comment onto one space after its hash run and two columns clear of the code beside it."
related : [align-comments, blank-lines, step-narration, line-overflow]
layout  : doc
---

# comment-spacing

<RuleLayout rule="comment_spacing">

`comment-spacing` settles a comment's opener onto a single space after the hash run and holds a trailing comment at least two columns clear of the code beside it, so the delimiter reads the same way on every line.

What separates deliberate indentation inside a comment from drift is whether the comment stands in a column. A comment opening at the same column as the one above or below it belongs to a laid-out block, so every indent inside that block survives:

```python
# options for the run:
#     --fast   skip the checks
#     --slow   run everything
```

A bare `#` sustains such a run rather than breaking it, since it opens at that column too, which is what keeps a spacer line from splitting one block into two. A comment sharing its column with nothing has no layout to protect, so its padded opener settles to the single space every other comment carries. A run opening on any whitespace other than a space collapses either way, a lone tab included, since no author draws that shape deliberately.

The space lands after the whole hash run rather than flattening it, leaving a `##` or `####` heading intact as the section marker it was written as, which is the same shape [[blank-lines]] reads when it decides a comment block divides its neighbors rather than describing the statement below. A run carrying no text after it is a divider rather than an opener, so a line of hashes alone passes through, and a comment carrying whitespace and no text sheds that whitespace down to a bare `#`. Where the run is followed by `!`, `:`, `'`, or `|`, the opener passes through untouched, covering the shebang line and Sphinx's `#:` attribute doc alongside the quoted and piped forms, whereas a trailing one still moves to the two-column floor, the exemption covering what a comment opens with rather than where it sits.

Two columns is the floor on a trailing comment rather than the target, so a wider gap survives as written and a shared comment column stays reachable. The gap is counted in characters, leaving a lone tab short of the floor and replaced with two spaces.

<Fixture rule="comment_spacing" case="hash_run_gains_one_space" />

<Fixture rule="comment_spacing" case="columnar_block_keeps_its_indents" />

<Fixture rule="comment_spacing" case="heading_run_keeps_its_hashes" />

<template #configuration>

<RuleConfigTable />

The opener shape and the two-column floor are both fixed, so the rule carries `enabled` as its only facet. A comment that must keep its own spacing takes an inline [**Suppression**](/usage/suppression) directive rather than a project-level knob, since the exception is per-comment rather than per-project.

</template>

</RuleLayout>
