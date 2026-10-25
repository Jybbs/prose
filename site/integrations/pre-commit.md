---
summary: 'Fails a commit that would land unformatted code, so nothing reaches the repository with layout drift.'
tagline: 'git staging'
---

# Pre-Commit

<Tool slug="precommit" /> runs *Prose* when you commit, so a staged file either matches the formatted layout or the commit fails. The hook runs on staged files only, checking the code you edited rather than the whole tree.

## Local Hook

Add a `local` hook to your `.pre-commit-config.yaml`:

```yaml
- repo: local
  hooks:
    - id       : prose
      name     : prose
      entry    : prose format
      language : system
      types    : [python]
      files    : '\.pyw?$'
```

`entry: prose format` rewrites the staged file in place, so pre-commit fails the hook with *"files were modified by this hook"* and leaves the rewritten file on disk for you to stage again and re-commit. This is the loop pre-commit runs for every hook that fixes files (*Black, isort, and autopep8 behave the same way*). For a hook that fails on pending rewrites without writing anything, use `entry: prose check` and run `prose format` yourself before committing again.

`language: system` tells pre-commit to run the `prose` binary already on your `PATH` (*see the [**Installation**](/guide/installation) chapter*) rather than installing a pinned copy into the hook's own environment. A project that prefers a pinned copy uses `language: python` with `additional_dependencies: ["prose-formatter==<version>"]`.

`files` keeps the hook to `.py` and `.pyw` files, since pre-commit's `python` type also matches an executable script whose shebang names Python, and `prose format` fails with exit code 4 on a named file outside the extensions it formats.

## Pairing With Ruff

A project that already runs `ruff format` runs it inside the *Prose* hook rather than in a hook of its own, because `ruff format` removes the padding *Prose* writes. Two hooks would each rewrite what the other just wrote, so both would fail on every commit, whereas one hook running `ruff format` ahead of `prose format` leaves a file unchanged once it holds their combined output:

```yaml
    - id       : prose
      name     : prose
      entry    : sh -c 'ruff format "$@" && prose format "$@"' --
      language : system
      types    : [python]
      files    : '\.pyw?$'
```

A file can fail the hook on two commits in a row, which the [**Ruff**](/integrations/ruff) integration page explains. A project that prefers pinned copies of both uses `language: python` with `additional_dependencies: ["prose-formatter==<version>", "ruff==<version>"]`.

## Upstream Hook

The [**Ruff pre-commit hook**](https://github.com/astral-sh/ruff-pre-commit) runs Ruff's linter without a system install, reading the `extend-ignore` list the [**Ruff**](/integrations/ruff) integration page gives so it does not report the whitespace *Prose* writes. Add this block above the `repo: local` block from the Local Hook section:

```yaml-vue
- repo: https://github.com/astral-sh/ruff-pre-commit
  rev: v{{ $frontmatter.ruffVersion }}
  hooks:
    - id: ruff-check
```

The same repository's `ruff-format` hook stays out, since it would remove the padding the *Prose* hook writes on every commit. `rev:` pins a Ruff release, and the version shown is the one *Prose* builds against, so the two stay in step without a separate pin.

## Exit Codes

The hook reads the same [**Exit Codes**](/reference/exit-codes) the CLI uses. A `format` hook exits 0 once its rewrites are written, yet pre-commit still fails it for modifying a file, as the Local Hook section describes. A `check` hook instead fails the commit on its exit code whenever a change is pending.
