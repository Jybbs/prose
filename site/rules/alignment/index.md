# Alignment Rules

The alignment rules pad the space before a shared token across consecutive rows so the token sits at one column, and the assignments, annotations, patterns, and trailing comments in a run then read as vertical groups. The column math lives in the [[aligner]] primitive, and each rule supplies the walker that reads which rows form a group. The per-rule `max-shift` facet limits how much padding one row may take, where each rule reads a run in source order and starts a new group at the first row whose gap from the narrowest row would exceed the limit. Setting it to `false` removes the limit, so a run of any width aligns on one column, and `0` forbids padding altogether. A `# prose: skip` keeps a row out of its group, so the rules read past a skipped single-line row and align the rows around it together, whereas a skipped multi-line statement ends the run.

<RuleCardList family="alignment" />

The [**Configuration**](/reference/configuration) reference lists the per-rule facets, and the [[aligner]] primitive covers the math the rules share. The [[colon-targets]] walker finds the `:` contexts the alignment rules read.
