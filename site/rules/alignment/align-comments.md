---
caption : "Aligns a run of consecutive trailing comments onto one shared column."
related : [comment-spacing, align-equals, align-colons, line-overflow, strip-align-padding]
layout  : doc
---

# align-comments

<RuleLayout rule="align_comments">

A trailing comment starts wherever its line of code happens to stop, so a block of annotated lines opens its notes at a different offset on every row even after the assignments beside them have settled into a column. The eye then tracks two ragged edges instead of one, and the notes read as a scatter rather than as the second column they are. `align-comments` gathers a run of consecutive trailing comments onto one shared offset, so the annotations read straight down beside the code they describe.

The rule walks the trailing comments in source order and grows a run while each next comment sits on the line directly below the last. A line carrying no trailing comment, a blank line, and an own-line comment each end the run, and a run whose rows open at differing indents realizes no column at all, since a shared offset would land where neither row's code actually ends. The shared column sits [[comment-spacing]]'s two-space gap past the widest row in the run, which keeps every aligned row at or above the PEP 8 floor.

<Fixture rule="align_comments" case="run_shares_one_comment_column" />

A row that reaches no shared column takes the two-space floor instead, which is what settles a hand-set gap lining up with nothing. That covers a lone trailing comment, since a run of one has no column to answer to, and it covers a row partitioning out of a run. Two caps drive that partition, the `max-shift` spread budget the family shares and the `code-line-length` budget, wherein a row whose aligned line would cross the budget stays where it sits rather than manufacturing an overflow for [[line-overflow]] to flag.

<Fixture rule="align_comments" case="bracket_rows_share_a_column" />

The column resolves to two past the widest row whether that pulls a comment right or left, so a run whose notes already line up at a wider gutter tightens onto that same offset. The alignment the author drew survives, with the slack past it removed the way [[strip-align-padding]] removes padding that lines up with nothing.

<Fixture rule="align_comments" case="uniform_gutter_tightens_to_the_floor" />

<Fixture rule="align_comments" case="long_line_holds_the_run_apart" />

A `# prose: skip` on a row holds it out of the column math without ending the run, so the rows above and below it reach across and settle together. The directive is itself a trailing comment, which is what lets a single annotation both name the exception and carry it.

<template #configuration>

<RuleConfigTable />

`max-shift` bounds how far a comment may shift to reach the shared column. The rule walks each run in source order and grows a column while its width spread stays within the cap, cutting a fresh column at the first row that would exceed it. A `max-shift` of `false` lifts the cap so a contiguous run folds into one column, and `0` forbids any shift, leaving every trailing comment at the two-space floor. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
