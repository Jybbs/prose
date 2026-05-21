---
category : auto-fix
family   : formatting
caption  : "normalizes blank-line counts to canonical values between thematically adjacent statements."
related  : [alphabetize, align-imports, bare-import-allowlist]
layout   : doc
---

# blank-lines

<RuleLayout rule="blank_lines" canonical="compound_then_def">

Blank lines carry rhythm. They tell the reader where one unit ends and the next begins, and a consistent rhythm across a file lets the reader skim by section without parsing each statement. `blank-lines` normalizes the discipline around module-level definitions, class members, and the `if __name__ == "__main__":` guard, so every file in the project reads with the same cadence.

Module-level `def` and `class` carry two blank lines before them, methods inside a class body carry one, a module-level statement after `if __name__ == "__main__":` carries one, and adjacent bare-import and `from`-import groups carry one between them. Inside function bodies the rule leaves blank-line discipline alone, since the in-body rhythm remains a per-author choice. The import surface sits downstream of [[alphabetize]] (*which orders the entries first*) and [[bare-import-allowlist]] (*which decides which packages keep the bare form*), then this rule lands the blank-line separators between groups, and [[align-imports]] closes the sequence by aligning the `import` keyword.

<template #canonical-lead>

Two blank lines precede every module-level `def` and `class`, giving the reader's eye an anchor between top-level units.

</template>

<template #more-examples>

<Fixture rule="blank_lines" case="bare_then_from_insert" title="One Blank Lands between Adjacent Bare and `from` Import Groups" />

<Fixture rule="blank_lines" case="class_with_docstring" title="The First Method after a Class Docstring Tightens Up" />

<Fixture rule="blank_lines" case="bare_then_from_collapse" title="Excess Blanks Collapse to the Canonical Single Blank" />

<Fixture rule="blank_lines" case="banner_comments" title="Banner Comments Don't Block the Blank-Line Discipline" />

<Fixture rule="blank_lines" case="comment_with_blank_above_def" title="Comments Attached to a `def` Keep Their Blank-Above" />

<Fixture rule="blank_lines" case="methods_in_class" title="Methods inside a Class Body Carry One Blank between Them" />

<Fixture rule="blank_lines" case="decorated_def" title="Decorators Travel with Their `def`, Not above the Blank Run" />

<Fixture rule="blank_lines" case="main_guard" title="The `__main__` Guard Carries One Blank above Its Sibling" />

<Fixture rule="blank_lines" case="bare_then_from_idempotent" title="Already-Conforming Source Is Left Alone" />

</template>

</RuleLayout>
