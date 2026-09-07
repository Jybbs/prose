---
summary: 'Fails a PR check when a file would change, with annotations inline on the diff and SARIF upload to the Security tab.'
tagline: 'CI runner · code scanning'
---

# GitHub Actions

*Prose* runs on the standard `ubuntu-latest` runner. The install step downloads the wheel through <Tool slug="uv" />, the check step runs `prose check`, and the exit code decides whether the job passes. The three jobs below differ in how much they show on the PR: a bare check, annotations inline on the diff, or a SARIF upload to [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning).

## Job Skeleton

Every job below plugs its `prose check` step into the same skeleton, which checks out the repository, installs `uv`, and installs the wheel:

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

`actions/checkout` puts the source on the runner, `astral-sh/setup-uv` installs `uv` and keeps its download cache between runs, and the last two steps install *Prose* and run the check. Each job below replaces the final `run` line.

## Minimal Check

The minimal job fails the workflow on any pending rewrite or lint finding and shows nothing on the PR beyond the pass or fail badge. The [**Exit Codes**](/reference/exit-codes) reference lists which exits count as failure:

```yaml
- run: prose check .
```

## Workflow Command Annotations

The `github` output format prints one [**workflow command**](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) per finding, which GitHub renders as an annotation on the PR diff beside the line it concerns:

```yaml
- run: prose check --output-format github .
```

The [**Output Formats**](/reference/output-formats) reference covers the record format, and the [**CLI Reference**](/reference/cli) covers `--output-format` and its default.

## SARIF Upload

The `sarif` output format writes findings to a file that a second step uploads through GitHub's CodeQL action, so they persist across runs and appear in the repository's Security tab under [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning):

```yaml
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

Each SARIF record carries the rule slug and the source location, so the Security tab keeps a history per rule. The [**Output Formats**](/reference/output-formats) reference lists the fields of each record.

## Persisting the Cache

Every run uses the per-user [**cache**](/reference/cache) by default, but a runner's filesystem is discarded after each job. `actions/cache` keeps `~/.cache/prose` between runs, so a file that has not changed costs a stat, a hash, and a deserialize instead of a full format:

```yaml
- uses: actions/cache@v4
  with:
    path: ~/.cache/prose
    key: prose-${{ runner.os }}-${{ hashFiles('prose.toml', '.config/prose.toml', 'pyproject.toml') }}
- run: prose check .
```

The key changes whenever a config file changes, so a configuration edit starts from an empty cache. A macOS runner uses `~/Library/Caches/prose` and a Windows runner `%LOCALAPPDATA%\prose\cache`, both listed on the [cache page](/reference/cache#location).

## Pairing With Ruff in CI

A project that runs [**Ruff**](https://docs.astral.sh/ruff/) too runs both tools as check steps, each failing the job on a pending rewrite without writing to the runner's disk:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff format --check .
- run: prose check .
```

The [**Ruff**](/integrations/ruff) integration page covers the `pycodestyle` codes to turn off in Ruff's linter and why Ruff's step comes first.

## Exit Codes

A CI job reads the same [**Exit Codes**](/reference/exit-codes) the CLI documents. Any non-zero exit fails the step unless `continue-on-error` is set, so a single `prose check` step is a complete gate.

The [**Pre-Commit**](/integrations/pre-commit) page covers running *Prose* at the commit boundary as well, and the [**Ruff**](/integrations/ruff) page covers running Ruff alongside it.
