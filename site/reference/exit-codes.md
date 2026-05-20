# Exit Codes

Every `prose check` and `prose format` invocation resolves into one of **five** exit codes that CI gates compile against. The codes are mutually exclusive at run time, in that when two outcomes apply, the higher number wins. A `format` run that auto-fixes a rule's diagnostics returns `0` once the rewrite lands *(the diagnostic was applied, not left pending)*, whereas a `check` run on the same source returns `1`. Parse failures on a single file surface as exit code `3` for that file, leaving the rest of the walked tree to settle independently, meaning one broken module never aborts the whole run.

<ExitCodeMatrix />

## CI Gating

The standard CI shape is a single `prose check` step where any non-zero exit fails the gate. For projects that want to gate on auto-fix and lint separately, the `2` code can be branched independently from `1` since lint diagnostics never auto-fix and so never resolve themselves.

```yaml
- run: uv tool install prose-formatter
- run: prose check .
```

The [**GitHub Actions**](/integrations/github-actions) integration page covers the workflow-command and SARIF shapes that surface diagnostics inline alongside the gate.

## Composition with Ruff

In the [**two-stage pipeline**](/guide/two-stage-pipeline), each tool returns its own exit code from its own invocation, and a non-zero from either step fails the gate without further wiring. The codes don't compose into a combined status, so the failure surface tells the developer which pass surfaced the diagnostic.

## Help Output

The CLI's `--help` prints the same matrix beneath the option list, so a developer scanning `prose check --help` sees the gating semantics without leaving the terminal.

For the per-subcommand flag list that produces each exit code, see the [**CLI Reference**](/reference/cli).
