---
caption : "Normalizes blank-line counts to canonical values between thematically adjacent statements."
related : [alphabetize-siblings, align-imports, bare-imports]
layout  : doc
---

# space-statements

<RuleLayout rule="space_statements">

Blank lines carry rhythm, telling the reader where one unit ends and the next begins, with a consistent rhythm across a file letting the reader skim by section without parsing each statement. `space-statements` normalizes the discipline around module-level definitions, class members, and the `if __name__ == "__main__":` guard, so every file in the project reads with the same cadence.

Module-level `def` and `class` carry two blank lines before them and two after, whatever top-level statement follows. Methods inside a class body carry one, a module-level statement after `if __name__ == "__main__":` carries one, adjacent bare-import and `from`-import groups carry one between them, and the first statement below the import block carries one. Inside function bodies the rule leaves blank-line discipline alone, since the in-body rhythm remains a per-author choice. A description-shaped own-line comment block above a statement binds tight against the following statement, reading the comment as a description of the statement it precedes, and it binds whether or not the author left a blank line between the two, so the reordering rules move the comment with the statement it heads. A block that holds its own line instead keeps 1 blank line below it to read as a divider, covering any line carrying a decorative rule of `=`, `-`, `*`, `_`, `#`, or `~`, a Markdown-style heading opening with two or more `#`, or a suppression directive. The canonical above-gap is measured from the topmost comment in the block either way. On the import surface this rule reads an order [[group-imports]] and [[alphabetize-siblings]] have already settled, lands the blank-line separators between groups, and leaves [[align-imports]] to align the `import` keyword afterward. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each sits.

<template #configuration>

<RuleConfigTable />

The canonical blank-line counts are hard-coded to PEP 8's `2`-between-top-level and `1`-between-methods cadence, so the rule carries `enabled` as its only facet. Projects that want a different cadence can disable the rule and let their editor's blank-line conventions stand.

</template>

</RuleLayout>
