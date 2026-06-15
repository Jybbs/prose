---
caption : "normalizes blank-line counts to canonical values between thematically adjacent statements."
related : [alphabetize, align-imports, bare-imports]
layout  : doc
---

# blank-lines

<RuleLayout rule="blank_lines">

Blank lines carry rhythm, telling the reader where one unit ends and the next begins, with a consistent rhythm across a file letting the reader skim by section without parsing each statement. `blank-lines` normalizes the discipline around module-level definitions, class members, and the `if __name__ == "__main__":` guard, so every file in the project reads with the same cadence.

Module-level `def` and `class` carry two blank lines before them and two after when followed by a top-level assignment. Methods inside a class body carry one, a module-level statement after `if __name__ == "__main__":` carries one, and adjacent bare-import and `from`-import groups carry one between them. Inside function bodies the rule leaves blank-line discipline alone, since the in-body rhythm remains a per-author choice. A description-shaped own-line comment block above a statement binds tight against the following statement, reading the comment as a description of the statement it precedes, whereas a banner-shaped block *(with any line a decorative rule of `=`, `-`, `*`, `_`, `#`, or `~`)* keeps 1 blank line below to read as a section divider. The canonical above-gap is measured from the topmost comment in the block either way. The import surface sits downstream of [[alphabetize]] (*which orders the entries first*) and [[bare-imports]] (*which decides which packages keep the bare form*), then this rule lands the blank-line separators between groups, and [[align-imports]] closes the sequence by aligning the `import` keyword.

<template #configuration>

<RuleConfigTable />

The canonical blank-line counts are hard-coded to PEP 8's `2`-between-top-level and `1`-between-methods cadence, so the rule carries `enabled` as its only knob. Projects that want a different cadence can disable the rule and let their editor's blank-line conventions stand.

</template>

</RuleLayout>
