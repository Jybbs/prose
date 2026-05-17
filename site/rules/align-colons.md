---
category: auto-fix
---

# align-colons

The `:` separator appears in **four contexts** where columns of values sit beside columns of names, and in each one the reader's eye wants a tidy table rather than a ragged margin. *Align-colons* gathers those contexts into a single shared alignment surface, so dictionary keys, dataclass and Pydantic fields, function-signature parameter annotations, and docstring `Args:` blocks all read as parallel two-column entries. Single-expression `match` arms live in a separate dispatch table owned by [**`match-case-align`**](/rules/match-case-align).

The rule walks each context independently, treating a group as the consecutive members sharing the same indentation level and parent shape. A blank line, a comment, or a non-member statement resets the group. Alignment honors the [**`singleton-rule`**](/rules/singleton-rule) so that one-member contexts skip padding altogether, leaving a one-key dict reading as plain code instead of a one-row table.

## Configuration

<AlignmentConfig />

## The Canonical Case

A dictionary literal with three entries of differing key lengths aligns on the `:` separator, and the reader reads keys and values as separate columns.

<Fixture rule="align_colons" case="dict_literal" />

## More Examples

<Fixture rule="align_colons" case="function_signature" title="Function-Signature Annotations Align Inside the Parameter List" />

<Fixture rule="align_colons" case="docstring_args_inline_phrase" title="Docstring Args Blocks Align the Parameter-Name Column" />

<Fixture rule="align_colons" case="dict_mixed_keys" title="Mixed Key Shapes Still Find the Widest Key" />

<Fixture rule="align_colons" case="dict_nested" title="Nested Dicts Align Independently at Each Level" />

<Fixture rule="align_colons" case="dict_unpacking" title="Unpacking Entries Break the Alignment Run" />

## Related

`:` is one of four separator surfaces the alignment engine runs across. [**`align-equals`**](/rules/align-equals) covers the `=` sign on consecutive assignments. [**`align-imports`**](/rules/align-imports) covers the `import` keyword on `from ... import ...` runs. [**`match-case-align`**](/rules/match-case-align) covers the post-pattern `:` on single-expression case bodies inside a `match`. The [**`singleton-rule`**](/rules/singleton-rule) drops the padding when a `:`-shaped group collapses to one member, so a one-key dict reads as plain code.

Upstream of this rule, [**`collection-layout`**](/rules/collection-layout) decides whether a dict literal spans one line or many, and [**`alphabetize`**](/rules/alphabetize) settles dataclass-field order before the alignment fires.
