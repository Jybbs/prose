---
description: "Runs `prose format` or `prose check` from an editor save, a pre-commit hook, and a CI job."
---

Every integration on the pages below runs `prose format` or `prose check` at one point in the development loop. The editor runs it on save, the pre-commit hook runs it on commit, and the CI workflow runs it before merge. Each one reads the same [`[tool.prose]`](/reference/configuration) table, so a rule turned off there is off at every point, and each reports the same [exit codes](/reference/exit-codes).

## Pick Your Boundary

<IntegrationCardGrid />

## The Boundaries Together

The three points *(save, commit, merge)* complement one another, each catching what the one before it missed:

1. Format-on-save fixes layout as soon as it drifts, so the working tree is already formatted before anything is staged.
2. The pre-commit hook catches a change that arrived without passing through the editor integration *(a patch applied with `git apply`, a teammate's unformatted edit pulled in)*.
3. The CI gate catches everything else, including pushes from contributors who run neither local hook.

## See Also

The [**Guide**](/guide/) section and the [**CLI Reference**](/reference/cli) cover the commands and flags every integration runs. [**Output Formats**](/reference/output-formats) covers the diagnostic formats an integration can read, and [**Rules**](/rules/) lists the rules every integration runs.
