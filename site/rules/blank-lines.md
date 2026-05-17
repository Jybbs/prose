---
category: auto-fix
related : [alphabetize, align-imports, bare-import-allowlist]
---

# blank-lines

Blank lines carry rhythm. They tell the reader where one unit ends and the next begins, and a consistent rhythm across a file lets the reader skim by section without parsing each statement. *Blank-lines* normalizes the discipline around module-level definitions, class members, and the `if __name__ == "__main__":` guard, so every file in the project reads with the same cadence.

Module-level `def` and `class` carry two blank lines before them, methods inside a class body carry one, a module-level statement after `if __name__ == "__main__":` carries one, and adjacent bare-import and `from`-import groups carry one between them. Inside function bodies the rule leaves blank-line discipline alone, since the in-body rhythm remains a per-author choice. The import surface sits downstream of [[alphabetize]] (*which orders the entries first*) and [[bare-import-allowlist]] (*which decides which packages keep the bare form*), then this rule lands the blank-line separators between groups, and [[align-imports]] closes the sequence by aligning the `import` keyword.

## Configuration

<ToggleConfig />

## The Canonical Case

Two blank lines precede every module-level `def` and `class`, giving the reader's eye an anchor between top-level units.

<Fixture rule="blank_lines" case="compound_then_def" />

## More Examples

<Fixture rule="blank_lines" case="bare_then_from_insert" title="One Blank Lands Between Adjacent Bare and `from` Import Groups" />

<Fixture rule="blank_lines" case="class_with_docstring" title="The First Method After a Class Docstring Tightens Up" />

<Fixture rule="blank_lines" case="bare_then_from_collapse" title="Excess Blanks Collapse to the Canonical Single Blank" />

<Fixture rule="blank_lines" case="banner_comments" title="Banner Comments Don't Block the Blank-Line Discipline" />

<Fixture rule="blank_lines" case="comment_with_blank_above_def" title="Comments Attached to a `def` Keep Their Blank-Above" />

<Fixture rule="blank_lines" case="methods_in_class" title="Methods Inside a Class Body Carry One Blank Between Them" />

<Fixture rule="blank_lines" case="decorated_def" title="Decorators Travel With Their `def`, Not Above the Blank Run" />

<Fixture rule="blank_lines" case="main_guard" title="The `__main__` Guard Carries One Blank Above Its Sibling" />

<Fixture rule="blank_lines" case="bare_then_from_idempotent" title="Already-Conforming Source Is Left Alone" />

## Related

The blank-line discipline composes with three other rules across the same import surface.

- [[alphabetize]] sorts the imports within each group, before the blank-line pass runs.
- [[align-imports]] aligns the `import` keyword across the sorted blocks.
- [[bare-import-allowlist]] rewrites bare-versus-from, which decides the group split that the one-blank rule then enforces.

For the underlying motivation, the [**Configuration**](/guide/configuration) chapter walks the per-rule defaults that govern every blank-line decision.
