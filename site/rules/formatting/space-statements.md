---
caption : "Sets the blank-line count between module-level definitions, class members, import groups, and the `__main__` guard to PEP 8's canonical values."
related : [alphabetize-siblings, align-imports, bare-imports]
layout  : doc
---

# space-statements

<RuleLayout rule="space_statements">

`space-statements` sets the number of blank lines around module-level definitions, class members, import groups, and the `if __name__ == "__main__":` guard, so every file in the project reads with the same rhythm. Blank lines tell the reader where one unit ends and the next begins, and a consistent count across a file lets the reader skim by section without parsing each statement.

A module-level `def` or `class` takes two blank lines before it and two after, whatever top-level statement follows. One blank line is the gap everywhere else the rule reaches:

1. A method inside a class body, and the first member below a class header or its docstring.
2. A module-level statement after `if __name__ == "__main__":`.
3. The boundary between adjacent bare-import and `from`-import groups.
4. The first statement below the import block.

Inside a function body the rule leaves the blank lines alone, since the in-body rhythm stays a per-author choice.

An own-line comment block that reads as a description of the statement below it binds tight against that statement, with no blank line between them whether or not the author left one, so the reordering rules move the comment with the statement it heads. A block that anchors in place instead of binding keeps one blank line below it, so it reads as a divider, and the canonical gap above is measured from the topmost comment in the block either way.

A block counts as a divider where any of its lines carries one of these:

- A decorative rule of `=`, `-`, `*`, `_`, `#`, `~`, `─`, `━`, or `═`.
- A Markdown-style heading opening with two or more `#`.
- A suppression directive.
- A tool pragma.

A block opening the file is spaced differently, in that the blank lines above it are removed entirely and the blank lines below it are capped at one. That one blank line is present where the block is a divider or where the author already left one and absent otherwise, so a module leading with a license header keeps the spacing it was written with and a module leading with a banner gains one blank line.

On the import block this rule reads an order [[group-imports]] and [[alphabetize-siblings]] have already settled, writes the blank-line separators between groups, and leaves [[align-imports]] to align the `import` keyword afterward. The [**Pipeline Order**](/reference/pipeline-order) reference lists where each sits.

<template #configuration>

<RuleConfigTable />

The canonical blank-line counts are fixed to PEP 8's `2`-between-top-level and `1`-between-methods cadence, so the rule carries `enabled` as its only facet. A project on a different cadence disables the rule and keeps its editor's blank-line conventions.

</template>

</RuleLayout>
