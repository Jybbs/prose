---
category : auto-fix
domain   : formatting
caption  : "*Prose* strips padding from alignment groups that resolve to a single member."
related  : [align-colons, align-equals, align-imports, match-case-align]
---

# singleton-rule

An alignment group exists to give the reader's eye a column to drop down. With **two or more members** the column carries information, where each row reads as a row in a table. With **exactly one member** the column becomes a single cell, and padding it to a width that no sibling matches adds visual noise without payoff. `singleton-rule` strips the pre-`:` padding from every `:`-alignment context that resolves to a single member, so a one-key dict, a one-arg signature, or a one-field dataclass reads as **plain code** instead of a one-row table.

The rule operates on the four `:`-shaped contexts that [[align-colons]] covers (*dict literals, dataclass and Pydantic fields, function-signature annotations, docstring `Args:` blocks*) plus the single-expression `match`-arm context that [[match-case-align]] covers. Multi-member groups whose `:`s sit on distinct lines pass through this rule untouched, since the colon-alignment surfaces own them. The `=`-alignment from [[align-equals]] and the `import`-keyword alignment from [[align-imports]] carry their own one-member fallbacks and don't need pruning here.

## Configuration

<RuleConfigTable preset="toggle" />

## The Canonical Case

A one-key dict literal drops its pre-`:` padding, reading as a plain key-value pair rather than a one-row table.

<Fixture rule="singleton_rule" case="dict_literal" />

## More Examples

<Fixture rule="singleton_rule" case="function_signature" title="A One-Arg Function Signature Drops the Parameter Padding" />

<Fixture rule="singleton_rule" case="match_case" title="A One-Arm `match` Drops the Pre-`:` Padding" />

<Fixture rule="singleton_rule" case="docstring_args" title="A One-Arg Docstring `Args:` Block Drops the Colon Padding" />

<Fixture rule="singleton_rule" case="method_self_and_kwarg" title="The `self` Receiver Doesn't Count Toward the Group Size" />

<Fixture rule="singleton_rule" case="mixed_singleton_and_group" title="Singleton and Multi-Member Groups Compose inside One File" />

<Fixture rule="singleton_rule" case="multiline_signature" title="Multi-Line Singleton Signatures Drop the Padding Too" />

<Fixture rule="singleton_rule" case="idempotent" title="Already-Conforming Source Is Left Alone" />

## Related

<RelatedRulesInline />

