# Rules

*Prose* ships two categories of rule, where an auto-fix rule rewrites source under `prose format` and reports each rewrite as a `Severity::Format` diagnostic under `prose check`, whereas a lint rule reports a `Severity::Lint` diagnostic under both subcommands and never rewrites.

The [**suppression directives**](/usage/suppression) silence any rule on a line or a block, and the [**`enabled`**](/reference/configuration#per-rule-facets) facet turns any rule off across a project without changing the rest of the pipeline.

<RulesPlate />

## Subsetting

`--select` and `--ignore` restrict the active set for one run and take precedence over the set the configuration enables. The [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter covers both flags.
