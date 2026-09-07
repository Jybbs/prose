---
caption : "Puts one space after a comment's hash run and at least two spaces between code and a trailing comment."
related : [align-comments, space-statements, step-narration, line-overflow]
layout  : doc
---

# normalize-comment-spacing

<RuleLayout rule="normalize_comment_spacing">

`normalize-comment-spacing` puts one space between a comment's hash run and its text, and widens the gap before a trailing comment to at least two spaces, so `x = 1 #note` reads `x = 1  # note` and the delimiter looks the same on every line.

Whether the indentation inside a comment is deliberate depends on whether the comment stands in a column. A comment opening at the same column as the comment above or below it belongs to a block the author laid out, so every indent inside that block stays:

```python
# options for the run:
#     --fast   skip the checks
#     --slow   run everything
```

A bare `#` continues such a block rather than ending it, since it opens at the same column, which keeps a spacer line from splitting one block into two. A comment sharing its column with nothing has no layout to keep, so a padded opener such as `#   note` is reduced to the single space every other comment takes. A run opening on any whitespace other than a space is reduced either way, a lone tab included, since no author indents a block with one.

The space goes after the whole hash run rather than shortening it, so a `##` or `####` heading keeps its hashes as the section marker it was written as. [[space-statements]] reads the same form when it treats a comment block as a divider rather than a description of the statement below. A run with no text after it is a divider rather than an opener, so a line of hashes alone passes through unchanged, and a comment carrying whitespace and no text drops that whitespace down to a bare `#`. Where the run is followed by `!`, `:`, `'`, or `|`, the opener passes through untouched, covering the shebang line and Sphinx's `#:` attribute doc alongside the quoted and piped forms. A trailing comment with such an opener still moves out to the two-space gap, because the exemption covers what a comment opens with rather than where it sits.

Two spaces is the minimum gap before a trailing comment rather than the target, so a wider gap stays as written and a comment column the author laid out is still there for [[align-comments]] to read. The gap is counted in characters, so a lone tab falls short and is replaced with two spaces.

<Fixture rule="normalize_comment_spacing" case="hash_run_gains_one_space" />

<Fixture rule="normalize_comment_spacing" case="columnar_block_keeps_its_indents" />

<Fixture rule="normalize_comment_spacing" case="heading_run_keeps_its_hashes" />

<template #configuration>

<RuleConfigTable />

The one-space opener and the two-space minimum are both fixed, so the rule carries `enabled` as its only facet. A comment that has to keep its own spacing takes an inline [**Suppression**](/usage/suppression) directive rather than a project-level key, since the exception is per-comment rather than per-project.

</template>

</RuleLayout>
