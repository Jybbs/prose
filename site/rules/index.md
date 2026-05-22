# Rules Overview

*Prose* ships **twenty** rules across two categories. Auto-fix rules rewrite source as part of `prose format` and surface as `Severity::Format` diagnostics under `prose check`. Lint rules surface as `Severity::Lint` diagnostics in both subcommands and never rewrite.

Every rule respects the [**suppression directives**](/guide/suppression) and the [**`enabled`**](/reference/configuration#per-rule-knobs) knob, which lets a project disable any rule without re-shaping the rest of the pipeline.

<RulesPlate />

## Subsetting

`--select` and `--ignore` restrict the active set per invocation, taking precedence over the configured-enabled set. See the [**Installation**](/guide/quick-start#subset-the-active-rules) chapter for the CLI surface.
