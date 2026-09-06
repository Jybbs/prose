---
caption : "Expands a single-line triple-quoted docstring so its opener, its body, and its closer each sit on a line of their own."
related : [wrap-docstrings, frame-docstrings]
layout  : doc
---

# expand-docstrings

<RuleLayout rule="expand_docstrings">

`expand-docstrings` rewrites every single-line triple-quoted docstring (*opener, body, and closer all on one line*) into the multi-line form, putting the opener, the body, and the closer each on a line of its own, so every documented unit in a project shows one structure. A single-line docstring reads as a kind of inline comment, and downstream tools (*Sphinx, IDE hover previews, doctest, PEP 257-aware linters*) treat it differently from its multi-line sibling.

The rule fires on module, class, and function single-line docstrings. The body text moves onto its own line at the docstring's indent with its leading and trailing whitespace trimmed and nothing else changed. [[frame-docstrings]] runs ahead of this rule and settles the quotes, so a requoted one-liner expands in the same pass, and [[wrap-docstrings]] then wraps the description prose against its budget.

The [[docstring]] walker reads against the PEP 257 definition, so an f-string (*`f"""..."""`*), a bytes literal (*`b"""..."""`*), and a concatenated string never count as docstrings and the rule skips them. A raw-prefixed (*`r"""`*) single-line docstring expands the same way as a plain one, with the prefix kept on the opener. A docstring whose body is empty or whitespace alone (*`""""""`*) stays as written, as does a non-triple-quoted one-liner and a docstring sharing the `def` line with its definition. The PEP 257 summary-line convention is out of scope for this rule, leaving the body's wording to authors and downstream conventions.

<template #related-after>

For the docstring budgets that govern wrapping, the [**Configuration**](/reference/configuration#docstring-budgets) chapter covers the description and structured line lengths.

</template>

</RuleLayout>
