---
description: "The rules that report a finding without rewriting the source."
---

# Lint Rules

Lint rules report a finding without rewriting the source. They run under both `prose check` and `prose format`, and any lint finding sets exit code `2`.

<RuleCardList family="lint" />

Each lint reports a pattern *Prose* can find but not resolve, because the right fix depends on a judgment the binary does not make. The [**Suppression**](/guide/suppression) chapter covers the per-line opt-out `# prose: ignore[<rule>]`, and the [**Exit Codes**](/reference/exit-codes) reference covers how the code gates a CI run.
