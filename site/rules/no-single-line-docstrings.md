---
category: auto-fix
related : [multi-line-docstrings, docstring-wrap]
---

# no-single-line-docstrings

A single-line docstring (*opener, body, and closer all on one line*) reads as a kind of inline comment, and many downstream tools (*Sphinx, IDE preview surfaces, doctest, PEP 257-aware linters*) treat it inconsistently with its multi-line sibling. *No-single-line-docstrings* expands every single-line triple-quoted docstring into the canonical multi-line shape, so a project's documentation surface presents one consistent structure across every documented unit.

The rule fires on module, class, and function single-line docstrings. The body content is preserved verbatim across the expansion, and the resulting multi-line form passes immediately to [[multi-line-docstrings]] for the opener-and-closer placement and to [[docstring-wrap]] for the line-budget wrap.

## Configuration

<ToggleConfig />

## The Canonical Case

A module-level single-line docstring expands to the multi-line form, with the opener and closer landing on their own lines.

<Fixture rule="no_single_line_docstrings" case="module_docstring" />

## More Examples

<Fixture rule="no_single_line_docstrings" case="class_docstring" title="Class Docstrings Expand the Same Way" />

<Fixture rule="no_single_line_docstrings" case="method_docstring" title="Method Docstrings Expand Inside the Class Body" />

<Fixture rule="no_single_line_docstrings" case="nested_def_docstrings" title="Nested Function Docstrings Expand at Each Scope" />

<Fixture rule="no_single_line_docstrings" case="single_to_multi" title="The Body Content Is Preserved Verbatim Across the Expansion" />

## Related

The docstring surface composes through two sibling rules that each shape a different aspect of the structure.

- [[multi-line-docstrings]] canonicalizes the opener and closer placement after this rule expands the body.
- [[docstring-wrap]] wraps the expanded description prose and structured sections to the configured budgets.

For the underlying budgets, the [**Configuration**](/guide/configuration#docstring-budgets) chapter walks the description and structured line lengths.
