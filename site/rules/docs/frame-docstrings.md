---
caption : "Canonicalizes a docstring's quotes and frames the opener and closer."
related : [wrap-docstrings, expand-docstrings]
layout  : doc
---

# frame-docstrings

<RuleLayout rule="frame_docstrings">

Python takes any string literal standing first in a module, class, or function as its docstring, whatever quotes surround it, and `frame-docstrings` canonicalizes every one to the `"""` form because the quotes are the docstring's frame. A `'''`-delimited docstring, a plain `'...'` or `"..."`, and an already-`"""` docstring all settle on the same triple-double-quote delimiter, a raw `r` prefix kept verbatim on the opener since PEP 257 sanctions `r"""` for a docstring carrying a backslash.

For a multi-line docstring the rule also lands the opening `"""` flush with the docstring indent on its own line and drops the closing `"""` to its own line beneath the last content line, leaving the prose body untouched between them. It runs ahead of [[expand-docstrings]], so a requoted one-liner expands to the multi-line shape in the same pass, and [[wrap-docstrings]] then wraps the description prose against its budget.

The walker [[docstring]] reads against the PEP 257 definition, so an f-string docstring *(`f"""..."""`)*, a bytes literal *(`b"""..."""`)*, and concatenated string forms are excluded by construction, Python assigning none of them a `__doc__`. When re-delimiting to `"""` would break the string, wherein the body already holds a `"""` run or a single-line body ends in `"`, the rule keeps the original quotes rather than corrupt it.

<template #related-after>

For the docstring budgets that govern wrapping, the [**Configuration**](/reference/configuration#docstring-budgets) chapter covers the description and structured line lengths.

</template>

</RuleLayout>
