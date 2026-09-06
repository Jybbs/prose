---
description: "Runs `prose format` or `prose check` from an editor save, a pre-commit hook, and a CI job."
---

# Integrations

Every integration on the pages below runs `prose format` or `prose check` at one point in the development loop. The editor runs it on save, the pre-commit hook runs it on commit, and the CI workflow runs it before merge. Each one reads the same [`[tool.prose]`](/reference/configuration) table and reports the same [exit codes](/reference/exit-codes), so adding a second integration is a configuration change rather than a new tool to learn.

## Pick Your Boundary

<IntegrationCardGrid />

## How the Boundaries Compose

The three points *(save, commit, merge)* complement one another, each catching what the one before it missed:

1. Format-on-save fixes layout as soon as it drifts, so the working tree is already formatted before anything is staged.
2. The pre-commit hook catches a change that arrived without passing through the editor integration *(a patch applied with `git apply`, a teammate's unformatted edit pulled in)*.
3. The CI gate catches everything else, including pushes from contributors who run neither local hook.

A project that wires all three runs the same `prose check` or `prose format` against the same [`[tool.prose]`](/reference/configuration) at every point, so a rule turned off in one place is off everywhere.

## See Also

The [**Usage**](/usage/) section and the [**CLI Reference**](/reference/cli) cover the commands and flags every integration runs. [**Exit Codes**](/reference/exit-codes) lists what each exit code means for a CI gate, [**Output Formats**](/reference/output-formats) covers the diagnostic formats an integration can read, and [**Rules**](/rules/) lists the rules every integration runs.
