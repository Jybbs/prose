---
category : auto-fix
family   : alignment
caption  : "aligns the `import` keyword across the unified bare-and-`from` import block."
related  : [align-colons, align-equals, alphabetize, bare-import-allowlist, blank-lines, match-case-align]
layout   : doc
---

# align-imports

<RuleLayout rule="align_imports" canonical="from_imports">

An import block carries two kinds of structure that the reader's eye wants to follow as columns. The module column says where a thing comes from, and the name column says what's pulled in. When the two columns float at varying widths, every line reads as a fresh sentence rather than a row in a table. `align-imports` gathers consecutive bare and `from` imports at the same indent into one unified block, right-aligning the `as` keyword in `import M as A` rows against the `import` keyword in `from M import N` rows so every post-keyword name lands at one shared column.

The rule reads each block as the run of consecutive imports at the same indent, absorbing a single blank line between adjacent imports so the kind-boundary cushion `blank-lines` introduces does not split the alignment. Two or more blank lines, or a non-import statement, end the block. Comments between imports flow through without resetting. Pair with [[alphabetize]] to sort entries within each block before alignment, with [[blank-lines]] to separate import groups by category, and with [[bare-import-allowlist]] to canonicalize bare-versus-from before the alignment pass.

<template #configuration>

<RuleConfigTable />

`max-shift` caps the per-line padding the alignment can introduce. When a block's widest module name would push the `import` keyword past the cap, `max-shift-policy` decides the fallback shape *(the `aliased_shift_limit_split`, `aliased_shift_limit_drop`, and `aliased_shift_limit_skip` fixtures above each demonstrate one of the three policies)*. The [**per-rule knobs**](/reference/configuration#per-rule-knobs) reference covers the full semantics.

</template>

<template #canonical-lead>

A run of `from ... import ...` statements lines up on the `import` keyword, so the module column flushes left and the name column flushes right.

</template>

<template #more-examples>

<Fixture rule="align_imports" case="unified_block" title="Bare Aliased and `from` Imports Share One `import` Column" />

<Fixture rule="align_imports" case="unified_from_then_bare" title="The Unified Block Holds Regardless of Kind Order" />

<Fixture rule="align_imports" case="aliased_imports" title="Bare Imports with Aliases Align on the `as` Keyword" />

<Fixture rule="align_imports" case="breakers" title="Non-Import Statements End the Block" />

<Fixture rule="align_imports" case="comment_breaks" title="Comments between Imports Flow through the Block" />

<Fixture rule="align_imports" case="two_blank_lines_break" title="Two Blank Lines End the Block at the First Such Gap" />

<Fixture rule="align_imports" case="aliased_shift_limit_split" title="A Widest Member past `max-shift` Splits the Group" />

<Fixture rule="align_imports" case="aliased_shift_limit_drop" title="The `drop` Policy Excludes the Widest Members" />

<Fixture rule="align_imports" case="aliased_shift_limit_skip" title="The `skip` Policy Leaves the Whole Group Unaligned" />

<Fixture rule="align_imports" case="idempotent" title="Already-Aligned Imports Are Left Alone" />

</template>

</RuleLayout>
