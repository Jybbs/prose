# GitHub Actions

*Prose* compiles cleanly against the standard `ubuntu-latest` runner. The install step fetches the wheel through <Tool slug="uv" />, the check step runs `prose check`, and the exit code drives the gate. The shapes below trade verbosity for richer surfacing on the PR diff: minimal check, inline workflow-command annotations, and SARIF upload for [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning).

## Job Skeleton

Every workflow shape below plugs its `prose check` step into the same skeleton, which checks out the repository, provisions `uv`, and installs the wheel. The full job reads:

```yaml
name: prose

on:
  pull_request:
  push:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v3
        with:
          enable-cache: true
      - run: uv tool install prose-formatter
      - run: prose check .
```

`actions/checkout` lands the source on the runner, `astral-sh/setup-uv` provisions `uv` and persists its download cache across runs, and the final two steps install *Prose* and run the check. The snippets below substitute their own `prose check` step into the last line.

## Minimal Check

Reach for the minimal shape when the gate's only job is to fail the workflow on any pending rewrite or lint diagnostic. The check runs against the canonical [**Exit Codes**](/reference/exit-codes) matrix, with no surfacing on the PR diff beyond the workflow's pass/fail badge:

```yaml
- run: prose check .
```

## Workflow Command Annotations

Reach for the workflow-command shape when the diagnostics should appear inline on the PR diff next to each offending line. The `github` output format emits [**workflow commands**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub renders as native check-run annotations:

```yaml
- run: prose check --output-format github .
```

The [**Output Formats**](/reference/output-formats) reference covers the record shape, and the [**CLI Reference**](/reference/cli) covers the `--output-format` flag's precedence and defaults.

## SARIF Upload

Reach for the SARIF shape when the project wants findings persisted across runs and surfaced in the repository's Security tab through [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning). The output goes to a file and an additional step uploads it through GitHub's CodeQL action:

```yaml
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

SARIF persists every diagnostic with its rule slug and source location, so the Security tab carries a tracked history per rule. The [**Output Formats**](/reference/output-formats) reference enumerates the per-finding record shape.

## Persisting the Cache

Repeat runs hit the user-level [**cache**](/reference/cache) on by default, but the runner's filesystem evaporates between jobs. Wire `actions/cache` to persist `~/.cache/prose` across runs, so an unchanged file collapses to a stat plus a hash plus a deserialize on every subsequent CI invocation:

```yaml
- uses: actions/cache@v4
  with:
    path: ~/.cache/prose
    key: prose-${{ runner.os }}-${{ hashFiles('pyproject.toml') }}
- run: prose check .
```

Keying off `pyproject.toml` invalidates the cache whenever configuration changes, since the cache key already digests the active `[tool.prose]` table and an upstream change to it produces a fresh set of entries. macOS runners use `~/Library/Caches/prose` and Windows runners use `%LOCALAPPDATA%\prose\cache`, both [documented on the cache page](/reference/cache#location).

## Pairing With Ruff in CI

When a project pairs *Prose* with [**Ruff**](https://docs.astral.sh/ruff/), the two tools chain into CI as sequential check steps, with Ruff first to settle line wraps and *Prose* second for layout. Each tool runs in check mode so the gate fails on any pending rewrite without writing to the runner's filesystem:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff format --check .
- run: prose check .
```

The [**Ruff**](/integrations/ruff) integration page covers the per-rule conflicts and the `extend-ignore` configuration that lets the two tools coexist.

## Exit Codes

CI gates compile against the same [**Exit Codes**](/reference/exit-codes) matrix the CLI publishes. A non-zero exit without `continue-on-error` fails the step. The common CI shape is a single `prose check` step wherein the exit code resolves the outcome cleanly.

For wiring *Prose* into the git commit boundary alongside CI, see the [**Pre-Commit**](/integrations/pre-commit) integration page. For pairing *Prose* with Ruff, see the [**Ruff**](/integrations/ruff) integration page.
