---
category : auto-fix
domain   : docs
caption  : "*Prose* rewrites single-line docstrings into the multi-line form."
related  : [docstring-wrap, multi-line-docstrings]
---

# no-single-line-docstrings

A single-line docstring (*opener, body, and closer all on one line*) reads as a kind of inline comment, and many downstream tools (*Sphinx, IDE preview surfaces, doctest, PEP 257-aware linters*) treat it inconsistently with its multi-line sibling. `no-single-line-docstrings` expands every single-line triple-quoted docstring into the canonical multi-line shape, so a project's documentation surface presents one consistent structure across every documented unit.

The rule fires on module, class, and function single-line docstrings. The body content is preserved verbatim across the expansion, and the resulting multi-line form passes immediately to [[multi-line-docstrings]] for the opener-and-closer placement and to [[docstring-wrap]] for the line-budget wrap.

## Configuration

<RuleConfigTable preset="toggle" />

## The Canonical Case

A module-level single-line docstring expands to the multi-line form, with the opener and closer landing on their own lines.

<Fixture rule="no_single_line_docstrings" case="module_docstring" />

## More Examples

<Fixture rule="no_single_line_docstrings" case="class_docstring" title="Class Docstrings Expand the Same Way" />

<Fixture rule="no_single_line_docstrings" case="method_docstring" title="Method Docstrings Expand inside the Class Body" />

<Fixture rule="no_single_line_docstrings" case="nested_def_docstrings" title="Nested Function Docstrings Expand at Each Scope" />

<Fixture rule="no_single_line_docstrings" case="single_to_multi" title="The Body Content Is Preserved Verbatim Across the Expansion" />

## Related

<RelatedRulesInline />

For the docstring budgets that govern wrapping, the [**Configuration**](/reference/configuration#docstring-budgets) chapter covers the description and structured line lengths.
