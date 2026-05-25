# Integrations

Every integration on the pages below is a thin wrapper around `prose format` or `prose check`, wired into a different boundary in the development loop. The editor wraps the save event, the pre-commit hook wraps the staging boundary, the CI workflow wraps the merge gate. Each layer runs the same CLI against the same [`[tool.prose]`](/reference/configuration) table and surfaces the same [exit codes](/reference/exit-codes), so adopting a second integration is configuration rather than a new mental model.

## Pick Your Boundary

<IntegrationCardGrid />

## How the Boundaries Compose

Three editing boundaries *(save, commit, merge)* are complementary rather than redundant. Run-on-save catches layout drift the instant it appears, leaving the working tree clean before the developer thinks about staging. The pre-commit hook catches the case where a save fires without the editor integration *(an upstream patch applied with `git apply`, a teammate's edit pulled in unformatted)*. The CI gate catches every remaining case, including pushes from contributors who run none of the local hooks. A project that wires all three runs the same `prose check` or `prose format` against the same [`[tool.prose]`](/reference/configuration) at every layer, so a rule disabled in one place is disabled everywhere.

## See Also

For the CLI flags every integration aims, see [**Usage**](/usage/) and the [**CLI Reference**](/reference/cli). For the exit-code contract every CI integration compiles against, see [**Exit Codes**](/reference/exit-codes), and for the diagnostic shapes the integrations route, see [**Output Formats**](/reference/output-formats). For the rule catalog the integrations run, see [**Rules**](/rules/).
