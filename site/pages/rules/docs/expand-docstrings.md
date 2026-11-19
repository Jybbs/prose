---
caption : "Expands a single-line triple-quoted docstring so its opener, its body, and its closer each sit on a line of their own."
related : [wrap-docstrings, frame-docstrings]
layout  : doc
---

# expand-docstrings

<RuleLayout rule="expand_docstrings">

`expand-docstrings` rewrites every single-line triple-quoted docstring (*opener, body, and closer all on one line*) into the multi-line form, putting the opener, the body, and the closer each on a line of its own, so every documented unit in a project shows one structure. A single-line docstring reads as a kind of inline comment, and downstream tools (*Sphinx, IDE hover previews, doctest, PEP 257-aware linters*) treat it differently from its multi-line sibling.

The rule fires on module, class, and function single-line docstrings. The body text moves onto its own line at the docstring's indent with its leading and trailing whitespace trimmed and nothing else changed. [[frame-docstrings]] runs ahead of this rule and settles the quotes, so a requoted one-liner expands in the same pass.

A string the [[docstring]] walker skips stays as written, as the [**Docs Rules**](/rules/docs/) page lists. A raw-prefixed (*`r"""`*) single-line docstring expands the same way as a plain one, with the prefix kept on the opener. A docstring whose body is empty or whitespace alone (*`""""""`*) stays as written, as does a non-triple-quoted one-liner and a docstring sharing the `def` line with its definition.

</RuleLayout>
