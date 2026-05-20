# Lint Rules

The **four** lint rules surface diagnostics without rewriting source. They run under both `prose check` and `prose format`, returning exit code 2 when any lint diagnostic fires, and they never produce an [[edit]]. Lint coincides with its domain, in that every lint rule sits in the `lint` domain and every rule in the `lint` domain is a lint. The category-versus-domain distinction collapses cleanly to one landing.

<RuleCardGrid category="lint" />

Each lint surfaces a pattern *Prose* notices but won't itself resolve, because the right fix depends on intent the binary can't infer. The [**Suppression**](/guide/suppression) chapter covers per-line opt-outs via `# prose: ignore[<rule>]`. The [**Exit Codes**](/reference/exit-codes) reference covers the gating semantics. For the auto-fix companion landing, see the [**Auto-Fix Rules**](/rules/auto-fix/) page. For the per-rule `enabled` knob, see the [**Configuration**](/reference/configuration) reference.
