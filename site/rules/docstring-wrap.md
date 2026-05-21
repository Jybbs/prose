---
category : auto-fix
domain   : docs
caption  : "*Prose* wraps multi-line docstring bodies at the configured measure."
related  : [multi-line-docstrings, no-single-line-docstrings]
---

# docstring-wrap

A docstring carries two readings inside one triple-quoted region. The description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. The structured `Args:`, `Returns:`, and `Raises:` sections read as code-shaped tables, where the line budget matches the surrounding code's `code-line-length` (*88 by default*) so that argument annotations sit at the same column as the function body's expressions. `docstring-wrap` honors both budgets, wrapping description prose to the narrower line and structured sections to the wider one.

The rule reads `docstring-line-length` for the description budget, `code-line-length` for the structured budget, and `docstring-structured-policy` to override the structured budget when a project prefers a single narrower line across the whole docstring. Code blocks inside the description (*fenced or indented*) are preserved verbatim, since their layout is load-bearing. The two sibling docstring rules sit upstream of this one: [[no-single-line-docstrings]] expands single-line docstrings into the multi-line shape, then [[multi-line-docstrings]] lands the opener and closer on their own lines, and only then does this rule wrap the resulting body.

## Configuration

<RuleConfigTable preset="toggle" />

Description and structured budgets come from the top-level [**Configuration**](/reference/configuration#top-level-keys) keys: `docstring-line-length` (*default 76*), `code-line-length` (*default 88*), and `docstring-structured-policy` (*defaulting to `"code-line-length"`*) drive the column targets.

## The Canonical Case

Description prose wraps to `docstring-line-length`, with the existing paragraph structure preserved across the rewrap.

<Fixture rule="docstring_wrap" case="description_wrap" />

## More Examples

<Fixture rule="docstring_wrap" case="args_wrap" title="Structured `Args:` Sections Wrap to the Wider Budget" />

<Fixture rule="docstring_wrap" case="mixed_description_and_args" title="Mixed Description + Args Apply Both Budgets in Sequence" />

<Fixture rule="docstring_wrap" case="code_block_fenced" title="Fenced Code Blocks inside the Description Are Preserved" />

<Fixture rule="docstring_wrap" case="code_block_indented" title="Indented Code Blocks Are Preserved Too" />

<Fixture rule="docstring_wrap" case="list_preserved" title="Bulleted Lists inside the Description Are Preserved" />

<Fixture rule="docstring_wrap" case="class_docstring" title="Class Docstrings Wrap the Same Way as Function Docstrings" />

<Fixture rule="docstring_wrap" case="returns_wrap" title="Structured `Returns:` Sections Wrap to the Wider Budget" />

<Fixture rule="docstring_wrap" case="policy_docstring_line_length" title="The `docstring-line-length` Policy Overrides the Structured Budget" />

<Fixture rule="docstring_wrap" case="idempotent" title="Already-Wrapped Source Is Left Alone" />

## Related

<RelatedRulesInline />

For the budget semantics, the [**Docstring Budgets**](/reference/configuration#docstring-budgets) section of the Configuration chapter covers how the description and structured budgets interact.
