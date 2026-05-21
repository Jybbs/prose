---
category : auto-fix
domain   : alignment
caption  : "*Prose* aligns the `import` keyword across `from … import …` runs and the `as` keyword across `import … as …` runs."
related  : [align-colons, align-equals, alphabetize, bare-import-allowlist, blank-lines, match-case-align]
---

# align-imports

An import block carries two kinds of structure that the reader's eye wants to follow as columns. The module column says where a thing comes from, and the name column says what's pulled in. When the two columns float at varying widths, every line reads as a fresh sentence rather than a row in a table. `align-imports` gathers consecutive `from ... import ...` statements (or consecutive `import ... as ...` statements) into a shared column for the `import` (or `as`) keyword, leaving the module column flush left and the name column flush right.

The rule reads each block as the run of consecutive imports at the same indentation. A blank line, a comment, or a non-import statement resets the run. Pair with [[alphabetize]] to sort entries within each block before alignment, with [[blank-lines]] to separate import groups by category, and with [[bare-import-allowlist]] to canonicalize bare-versus-from before the alignment pass.

## Configuration

<RuleConfigTable preset="alignment" />

## The Canonical Case

A run of `from ... import ...` statements lines up on the `import` keyword, so the module column flushes left and the name column flushes right.

<Fixture rule="align_imports" case="from_imports" />

## More Examples

<Fixture rule="align_imports" case="aliased_imports" title="Bare Imports with Aliases Align on the `as` Keyword" />

<Fixture rule="align_imports" case="breakers" title="Non-Import Statements Reset the Alignment Run" />

<Fixture rule="align_imports" case="comment_breaks" title="Comment Lines Reset the Run, like Blank Lines" />

<Fixture rule="align_imports" case="aliased_shift_limit_split" title="A Widest Member past `max-shift` Splits the Group" />

<Fixture rule="align_imports" case="aliased_shift_limit_drop" title="The `drop` Policy Excludes the Widest Members" />

<Fixture rule="align_imports" case="aliased_shift_limit_skip" title="The `skip` Policy Leaves the Whole Group Unaligned" />

<Fixture rule="align_imports" case="idempotent" title="Already-Aligned Imports Are Left Alone" />

## Related

<RelatedRulesInline />
