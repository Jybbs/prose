---
description: "The rules that pad the space before a shared token so consecutive rows read as columns."
---

# Alignment Rules

The alignment rules pad the space before a shared token across consecutive rows so the token sits at one column, and the assignments, annotations, patterns, and trailing comments in a run then read as vertical groups. The column math lives in the [[aligner]] primitive, and each rule decides which rows form a group. The rules that align a `:` build their members through [[colon-targets]]. The per-rule `max-shift` facet limits how much padding one row may take, where each rule reads a run in source order and starts a new group at the first row that would push some row's padding past the limit. A `# prose: skip` holds a row out of its group, as [**Rewrite Suppression**](/reference/suppression-directives#rewrite-suppression) sets out.

<RuleCardList family="alignment" />
