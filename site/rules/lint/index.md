---
description: "The rules that report a finding without rewriting the source."
---

# Lint Rules

Lint rules report a finding without rewriting the source. They run under both `prose check` and `prose format`, they never produce an [[edit]], and any lint finding sets exit code `2`.

<RuleCardList family="lint" />

Each lint reports a pattern *Prose* can find but not resolve, because the right fix depends on a judgment the binary does not make. The [**Suppression**](/usage/suppression) chapter covers the per-line opt-out `# prose: ignore[<rule>]`, the [**Exit Codes**](/reference/exit-codes) reference covers how the code gates a CI run, the [**Rules**](/rules/) page lists every rule beside its family, and the [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` facet.
