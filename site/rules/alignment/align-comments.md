---
caption : "Pads the gap before each trailing comment in a run so every `#` sits at one shared column."
related : [normalize-comment-spacing, align-equals, align-colons, line-overflow, strip-stranded-padding]
layout  : doc
---

# align-comments

<RuleLayout rule="align_comments">

`align-comments` pads the gap before each trailing comment in a run so every `#` sits at one shared column, and the notes then read straight down beside the code they describe. Without it a trailing comment starts wherever its line of code stops, so a block of annotated lines opens its notes at a different offset on every row even after the assignments beside them share a column, and the eye tracks two ragged edges instead of one.

The rule reads the trailing comments in source order and extends a run while each next comment sits on the line directly below the last. A line with no trailing comment, a blank line, and an own-line comment each end the run, and a run whose rows sit at differing indents gets no column at all, since a shared column would sit where neither row's code ends. The shared column sits two spaces past the widest row in the run, the gap [[normalize-comment-spacing]] sets, which keeps every aligned row at or above the PEP 8 minimum.

<Fixture rule="align_comments" case="run_shares_one_comment_column" />

A row that reaches no shared column takes the two-space minimum instead, which is how a hand-set gap lining up with nothing settles. That covers a lone trailing comment, since a run of one has no column to share, and it covers a row that leaves its run. Two limits split a row out of a run, the `max-shift` limit every alignment rule carries and the `code-line-length` budget, where a row whose aligned line would cross the budget stays where it sits rather than creating an overflow for [[line-overflow]] to report.

<Fixture rule="align_comments" case="bracket_rows_share_a_column" />

The column resolves to two spaces past the widest row whether that moves a comment right or left, so a run whose notes already line up at a wider gutter tightens onto that same column. The alignment the author drew survives, with the slack past it removed the way [[strip-stranded-padding]] removes padding that lines up with nothing.

<Fixture rule="align_comments" case="uniform_gutter_tightens_to_the_floor" />

<Fixture rule="align_comments" case="long_line_holds_the_run_apart" />

A `# prose: skip` on a row keeps it out of the column math without ending the run, so the rows above and below it reach across it and align together. The directive is itself a trailing comment, which is what lets one annotation both name the exception and carry it.

<template #configuration>

<RuleConfigTable />

`max-shift` limits how much padding one comment may take to reach the shared column. The rule reads each run in source order and extends a column while the gap between the widest and narrowest rows stays within the limit, starting a new column at the first row that would exceed it. Setting `max-shift` to `false` removes the limit, so a run of any width aligns on one column, and `0` forbids padding altogether, leaving every trailing comment at the two-space minimum. The [**per-rule facets**](/reference/configuration#per-rule-facets) reference covers the full semantics.

</template>

</RuleLayout>
