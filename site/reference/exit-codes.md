# Exit Codes

Every `prose check` and `prose format` invocation resolves into a discrete exit code that CI gates compile against. The codes are mutually exclusive at run time, in that when two outcomes apply, the higher number wins. A `format` run that auto-fixes a rule's diagnostics returns `0` once the rewrite lands *(the diagnostic was applied, not left pending)*, whereas a `check` run on the same source returns `1`.

::: info Per-File Failures Stay Local
Parse failures on a single file surface as exit code `3` for that file, leaving the rest of the walked tree to settle independently.
:::

<ExitCodeMatrix />

## CI Gating

The standard CI shape is a single `prose check` step where any non-zero exit fails the gate. For projects that want to gate on auto-fix and lint separately, the `2` code can be branched independently from `1` since lint diagnostics never auto-fix and so never resolve themselves.

```yaml
- run: uv tool install prose-formatter
- run: prose check .
```

The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow-command and SARIF shapes that surface diagnostics inline alongside the gate.

## Composition with Ruff

When *Prose* is [**paired with Ruff**](/integrations/ruff), each tool returns its own exit code from its own invocation, and a non-zero from either step fails the gate without further wiring. The codes don't compose into a combined status, so the failure surface tells the developer which pass surfaced the diagnostic. When both passes fail the same workflow run, each step surfaces its own exit code in its own log group, in that the gate fails at the first non-zero step under the default sequential-step shape, whereas an `if: always()` clause on the *Prose* step lets both pass and exit codes surface side-by-side for a workflow that would rather see every failure at once.

## Help Output

The CLI's `--help` prints the same matrix beneath the option list, so a developer scanning `prose check --help` sees the gating semantics without leaving the terminal.

For the per-subcommand flag list that produces each exit code, see the [**CLI Reference**](/reference/cli).
