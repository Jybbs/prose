---
category: auto-fix
related : [docstring-wrap, no-single-line-docstrings]
---

# multi-line-docstrings

A multi-line docstring whose opener or closer shares a line with the body reads as a fragment, with the prose flowing into the triple-quotes rather than sitting between them as a self-contained block. *Multi-line-docstrings* lands the opening `"""` flush with the docstring indent on its own line, drops the closing `"""` to its own line beneath the last content line, and leaves the prose body untouched between them.

The rule fires on every multi-line docstring across module, class, and function scopes. Single-line docstrings (*opener, body, and closer all on one line*) are left alone for [[no-single-line-docstrings]] to handle. Pair with [[docstring-wrap]] for the description-prose wrap that runs after this rule canonicalizes the opener and closer.

## Configuration

<ToggleConfig />

## The Canonical Case

A docstring whose opener shares a line with the first body sentence drops the body to a new line beneath the opener.

<Fixture rule="multi_line_docstrings" case="opening_inline_body" />

## More Examples

<Fixture rule="multi_line_docstrings" case="closing_inline_body" title="A Closer Sitting on the Last Content Line Drops to Its Own Line" />

<Fixture rule="multi_line_docstrings" case="idempotent_multi" title="Already-Canonical Multi-Line Docstrings Are Left Alone" />

## Related

The docstring surface composes through two sibling rules that each shape a different aspect of the structure.

- [[no-single-line-docstrings]] expands single-line docstrings into the multi-line shape this rule canonicalizes.
- [[docstring-wrap]] wraps description prose and structured sections to the configured budgets, after this rule places the opener and closer.

For the underlying budgets that govern docstring wrapping, the [**Configuration**](/guide/configuration#docstring-budgets) chapter covers the description and structured line lengths.
