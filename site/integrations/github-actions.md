# GitHub Actions

*Prose* compiles cleanly against the standard `ubuntu-latest` runner. The install step fetches the wheel through <Tool slug="uv" />, the check step runs `prose check`, and the exit code drives the gate. Three surfaces cover the common shapes, wherein each one trades verbosity for richer surfacing on the PR diff: minimal check, inline workflow-command annotations, and SARIF upload for [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning).

## Minimal Check

The shortest viable workflow step:

```yaml
- run: uv tool install prose-formatter
- run: prose check .
```

`prose check` exits with the canonical [**Exit Codes**](/reference/exit-codes) matrix, so the gate fails on any pending rewrite or lint diagnostic without further wiring.

## Workflow Command Annotations

For inline annotations on the PR diff, the `github` output format emits [**workflow commands**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub renders as native check-run annotations next to each offending line:

```yaml
- run: uv tool install prose-formatter
- run: prose check --output-format github .
```

The [**Output Formats**](/reference/output-formats) reference covers the record shape, and the [**CLI Reference**](/reference/cli) covers the `--output-format` flag's precedence and defaults.

## SARIF Upload

For findings that persist across runs and surface in [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning), emit SARIF and upload it through GitHub's CodeQL action:

```yaml
- run: uv tool install prose-formatter
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

SARIF persists every diagnostic with its rule slug and source location, so the repository's Security tab carries a tracked history per rule. The [**Output Formats**](/reference/output-formats) reference enumerates the per-finding record shape.

## Two-Stage Pipeline in CI

The same two-stage pipeline that runs locally compiles into CI, with [**Ruff**](https://docs.astral.sh/ruff/) first to settle line wraps and *Prose* second for layout. Each tool runs in check mode so the gate fails on any pending rewrite without writing to the runner's filesystem:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff format --check .
- run: prose check .
```

The [**Ruff**](/integrations/ruff) integration page covers the per-rule conflicts and the `extend-ignore` configuration that lets the two tools coexist.

## Exit Codes

CI gates compile against the same [**Exit Codes**](/reference/exit-codes) matrix the CLI publishes. A non-zero exit without `continue-on-error` fails the step. The common CI shape is a single `prose check` step wherein the exit code resolves the outcome cleanly.

For wiring *Prose* into the git commit boundary alongside CI, see the [**Pre-Commit**](/integrations/pre-commit) integration page. For the canonical two-stage pipeline that runs Ruff alongside *Prose*, see the [**Ruff**](/integrations/ruff) integration page.
