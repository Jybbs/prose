# CI Integration

*Prose* compiles cleanly against any CI runner that has Python on `PATH`. The check-mode binary exits non-zero on any pending rewrite or lint diagnostic, so the standard run-and-fail pattern works without further wiring. Three surfaces (*GitHub Actions, SARIF Code Scanning, pre-commit*) cover the common pipelines, and the same exit-code matrix the local CLI publishes gates each one.

## On Save

Each CI surface wires *Prose* the same way: install the binary, invoke `prose check`, let the exit code drive the gate.

::: code-group

```yaml [GitHub Actions]
- run: uv tool install prose-formatter
- run: prose check .
```

```yaml [GitHub Actions (annotations)]
- run: uv tool install prose-formatter
- run: prose check --output-format github .
```

```yaml [SARIF Code Scanning]
- run: uv tool install prose-formatter
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

```yaml [pre-commit hook]
- repo: local
  hooks:
    - id      : prose
      name    : prose
      entry   : prose format
      language: system
      types   : [python]
```

```yaml [Two-stage pipeline]
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff format --check .
- run: prose check .
```

:::

The GitHub Actions annotation form (`--output-format github`) emits [**workflow commands**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub renders as native check-run annotations, so each diagnostic surfaces inline next to the offending line. The SARIF form persists findings across runs and surfaces them in the repository's Security tab via [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning), with a tracked history per rule. The pre-commit hook surfaces the same exit codes the CLI uses, so a `format` hook never fails on rewrites it applies and a `check` hook fails the commit when changes are pending.

## Exit Codes

CI gates compile against the same exit-code matrix the CLI publishes. The [**Installation**](/guide/installation#exit-codes) chapter carries the canonical interactive table. The common CI shape is a single `prose check` step where a non-zero exit fails the gate.

::: tip Composes with editor squiggles
The `--output-format json` path drives editor squiggles, while `--output-format github` and `--output-format sarif` drive CI annotations. All three consume the same diagnostic shape with the [[rule-id]] slug carried inside `code`, so a project's CI gate and its editor surface stay in sync without extra translation.
:::

## Two-Stage Pipeline in CI

The same two-stage pipeline that runs locally compiles into CI, with Ruff first to settle line wraps and *Prose* second for layout. Each tool runs in `--check` / `check` mode so the gate fails on any pending rewrite without writing to the runner's filesystem. The [**Installation**](/guide/installation#two-stage-pipeline) chapter walks the local equivalent and the rationale for the order.

For per-line opt-outs that survive CI runs, the [**Suppression**](/guide/suppression) chapter covers block markers, line markers, and lint directives. For wiring *Prose* into editor-side run-on-save loops alongside CI, the [**Editor Integration**](/guide/editor-integration) chapter covers VSCode, Neovim, JetBrains, Sublime Text, Emacs, Helix, and the structured-diagnostics surface.
