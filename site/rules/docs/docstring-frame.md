---
caption : "Drops a multi-line docstring's opener and closer onto their own lines."
related : [docstring-wrap, docstring-expand]
layout  : doc
---

# docstring-frame

<RuleLayout rule="docstring_frame">

A multi-line docstring whose opener or closer shares a line with the body reads as a fragment, with the prose flowing into the triple-quotes rather than sitting between them as a self-contained block. `docstring-frame` lands the opening `"""` flush with the docstring indent on its own line, drops the closing `"""` to its own line beneath the last content line, and leaves the prose body untouched between them.

The rule fires on every multi-line docstring across module, class, and function scopes. Single-line docstrings (*opener, body, and closer all on one line*) are left alone for [[docstring-expand]] to handle. Pair with [[docstring-wrap]] for the description-prose wrap that runs after this rule canonicalizes the opener and closer.

The walker [[docstring]] reads against the PEP 257 definition, so f-string docstrings *(`f"""..."""`)* and concatenated string forms are excluded by construction. Raw-prefixed *(`r"""`)* and byte-prefixed *(`b"""`)* literals canonicalize the same way as plain triple-quoted forms, with the prefix preserved verbatim on the opener. An empty docstring *(`""""""`)* lands as an empty multi-line shape, leaving the opener and closer on their own lines with a blank line between them.

<template #related-after>

For the docstring budgets that govern wrapping, the [**Configuration**](/reference/configuration#docstring-budgets) chapter covers the description and structured line lengths.

</template>

</RuleLayout>
