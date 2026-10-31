---
summary: 'Fails a PR check when a file would change, with annotations inline on the diff and SARIF upload to the Security tab.'
tagline: 'CI runner · code scanning'
---

# GitHub Actions

*Prose* runs on the standard `ubuntu-latest` runner, installed through <Tool slug="uv" />, and the jobs below differ in how much they show on the PR, from the pass or fail badge alone to annotations inline on the diff or a SARIF upload to [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning).

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

`astral-sh/setup-uv` keeps the `uv` download cache between runs. As written, the skeleton is the minimal job. Any non-zero exit fails a step unless `continue-on-error` is set, so a single `prose check` step is a complete gate, and the [**Exit Codes**](/reference/exit-codes) reference lists what each code means.

## Workflow Command Annotations

The `github` output format shows each finding as an annotation on the PR diff beside the line it concerns:

```yaml
- run: prose check --output-format github .
```

The [**`github`**](/reference/output-formats#github) section of the Output Formats reference covers the workflow commands it prints, and the [**CLI Reference**](/reference/cli) covers `--output-format` and its default.

## SARIF Upload

The `sarif` output format writes findings to a file that a second step uploads through GitHub's CodeQL action, so they persist across runs and appear in the repository's Security tab under [**Code Scanning**](https://docs.github.com/en/code-security/code-scanning):

```yaml
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v4
  if: ${{ !cancelled() }}
  with:
    sarif_file: prose.sarif
```

Each SARIF record carries the rule slug and the source location, so the Security tab keeps a history per rule. The upload needs the job's `permissions` to grant `security-events: write`. The step runs whatever `prose check` exits, so the findings reach the Security tab on the run whose gate fails.

## Persisting the Cache

Every run uses the per-user [**cache**](/reference/cache) by default, but a runner's filesystem is discarded after each job. `actions/cache` keeps `~/.cache/prose` between runs, so a file that has not changed is read from the cache instead of formatted again:

```yaml
- uses: actions/cache@v4
  with:
    path: ~/.cache/prose
    key: prose-${{ runner.os }}-${{ hashFiles('prose.toml', '.config/prose.toml', 'pyproject.toml') }}
- run: prose check .
```

The key changes whenever a config file changes, so a configuration edit starts from an empty cache. A macOS runner uses `~/Library/Caches/prose` and a Windows runner `%LOCALAPPDATA%\prose`, both listed on the [cache page](/reference/cache#location).

## Pairing With Ruff in CI

A project that already runs [**Ruff**](https://docs.astral.sh/ruff/) checks the layout through `prose check` and runs Ruff's linter beside it, since `ruff format --check` reports every row *Prose* pads as a pending change. Neither step rewrites a source file:

```yaml
- run: uv tool install ruff
- run: uv tool install prose-formatter
- run: ruff check .
- run: prose check .
```

Each tool exits with its own code from its own step, so a non-zero exit from either fails the job and the failing step names the tool that reported the finding. By default a step runs only once every step before it has succeeded, so where both would fail only the first shows. Setting `if: ${{ !cancelled() }}` on the `prose check` step runs it whatever the steps before it returned, and both results then appear in one run.

The [**Ruff**](/integrations/ruff) integration page covers the `pycodestyle` codes to turn off in Ruff's linter and why `ruff format` runs ahead of `prose format` wherever both write.

The [**Pre-Commit**](/integrations/pre-commit) page covers running *Prose* at the commit boundary as well.
