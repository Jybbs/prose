---
category : auto-fix
family   : alignment
caption  : "aligns the `:` separator across dict literals, dataclass and Pydantic fields, function-signature annotations, and docstring `Args:` blocks."
related  : [align-equals, align-imports, alphabetize, collection-layout, match-case-align, singleton-rule]
layout   : doc
---

# align-colons

<RuleLayout rule="align_colons" canonical="dict_literal">

The `:` separator appears in **four contexts** where columns of values sit beside columns of names, and in each one the reader's eye wants a tidy table rather than a ragged margin. `align-colons` gathers those contexts into a single shared alignment surface, so dictionary keys, dataclass and Pydantic fields, function-signature parameter annotations, and docstring `Args:` blocks all read as parallel two-column entries. Single-expression `match` arms live in a separate dispatch table owned by [[match-case-align]].

The rule walks each context independently, treating a group as the consecutive members sharing the same indentation level and parent shape. A blank line, a comment, or a non-member statement resets the group. Alignment honors the [[singleton-rule]] so that one-member contexts skip padding altogether, leaving a one-key dict reading as plain code instead of a one-row table.

<template #canonical-lead>

A dictionary literal with three entries of differing key lengths aligns on the `:` separator, and the reader reads keys and values as separate columns.

</template>

<template #more-examples>

<Fixture rule="align_colons" case="function_signature" title="Function-Signature Annotations Align inside the Parameter List" />

<Fixture rule="align_colons" case="docstring_args_inline_phrase" title="Docstring Args Blocks Align the Parameter-Name Column" />

<Fixture rule="align_colons" case="dict_mixed_keys" title="Mixed Key Shapes Still Find the Widest Key" />

<Fixture rule="align_colons" case="dict_nested" title="Nested Dicts Align Independently at Each Level" />

<Fixture rule="align_colons" case="dict_unpacking" title="Unpacking Entries Break the Alignment Run" />

</template>

</RuleLayout>
