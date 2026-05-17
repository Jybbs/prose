---
category: auto-fix
---

# singleton-rule

An alignment group exists to give the reader's eye a column to drop down. With **two or more members** the column carries information, where each row reads as a row in a table. With **exactly one member** the column becomes a single cell, and padding it to a width that no sibling matches adds visual noise without payoff. *Singleton-rule* strips the pre-`:` padding from every `:`-alignment context that resolves to a single member, so a one-key dict, a one-arg signature, or a one-field dataclass reads as **plain code** instead of a one-row table.

The rule operates on the four `:`-shaped contexts that [**`align-colons`**](/rules/align-colons) covers (*dict literals, dataclass and Pydantic fields, function-signature annotations, docstring `Args:` blocks*) plus the single-expression `match`-arm context that [**`match-case-align`**](/rules/match-case-align) covers. Multi-member groups whose `:`s sit on distinct lines pass through this rule untouched, since the colon-alignment surfaces own them. The `=`-alignment from [**`align-equals`**](/rules/align-equals) and the `import`-keyword alignment from [**`align-imports`**](/rules/align-imports) carry their own one-member fallbacks and don't need pruning here.

## Configuration

<ToggleConfig />

## The Canonical Case

A one-key dict literal drops its pre-`:` padding, reading as a plain key-value pair rather than a one-row table.

<Fixture rule="singleton_rule" case="dict_literal" />

## More Examples

<Fixture rule="singleton_rule" case="function_signature" title="A One-Arg Function Signature Drops the Parameter Padding" />

<Fixture rule="singleton_rule" case="match_case" title="A One-Arm `match` Drops the Pre-`:` Padding" />

<Fixture rule="singleton_rule" case="docstring_args" title="A One-Arg Docstring `Args:` Block Drops the Colon Padding" />

<Fixture rule="singleton_rule" case="method_self_and_kwarg" title="The `self` Receiver Doesn't Count Toward the Group Size" />

<Fixture rule="singleton_rule" case="mixed_singleton_and_group" title="Singleton and Multi-Member Groups Compose Inside One File" />

<Fixture rule="singleton_rule" case="multiline_signature" title="Multi-Line Singleton Signatures Drop the Padding Too" />

<Fixture rule="singleton_rule" case="idempotent" title="Already-Conforming Source Is Left Alone" />

## Related

This pass prunes one-member groups on the two `:`-shaped alignment surfaces: [**`align-colons`**](/rules/align-colons) (*dict literals, dataclass and Pydantic fields, function-signature annotations, docstring `Args:` blocks*) and [**`match-case-align`**](/rules/match-case-align) (*single-expression `match` arms*). The `=` and `import` surfaces ([**`align-equals`**](/rules/align-equals), [**`align-imports`**](/rules/align-imports)) carry their own one-member fallbacks and don't need pruning here.

For the underlying motivation, the landing page's [**reading metaphor**](/) frames why a one-row table reads worse than the plain expression it would otherwise be.
