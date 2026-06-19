# Formatting Rules

The formatting rules clear the small scaffolding that clutters a statement once its shape is settled, normalizing blank-line counts between definitions, shedding grouping parentheses that bind nothing, dropping a redundant `-> None`, stripping trailing commas, and removing a `from __future__ import annotations` that no longer earns its place on the target version. Each rewrite is narrower than a layout rule and more pervasive than an ordering rule, tidying what the eye reads without touching the structure it reads.

<RuleCardList family="formatting" />

For enabling or disabling any of these rules, see the [**Configuration**](/reference/configuration) reference. For the pipeline order they fire in, see the [**Pipeline Order**](/reference/pipeline-order) reference.
