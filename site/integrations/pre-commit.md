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
```

`entry: prose format` rewrites the staged file in place, so the hook fails the commit with a *"fixed by **Prose**"* diff and leaves the rewritten file on disk for you to stage again and re-commit. This is the loop pre-commit runs for every hook that fixes files *(Black, isort, and autopep8 behave the same way)*. For a hook that fails on pending rewrites without writing anything, use `entry: prose check` and run `prose format` yourself before committing again.

`language: system` tells pre-commit to run the `prose` binary already on your `PATH` *(see the [**Installation**](/usage/installation) chapter)* rather than installing a pinned copy into the hook's own environment. A project that prefers a pinned copy uses `language: python` with `additional_dependencies: ["prose-formatter==<version>"]`.

## Pairing With Ruff

A project that runs `ruff format` as well adds a Ruff hook above the *Prose* hook in the same `repo: local` block. pre-commit runs hooks in the order written, and Ruff's hook comes first because a later `ruff format` would remove the padding *Prose* writes:

```yaml
    - id       : ruff-format
      name     : ruff format
      entry    : ruff format
      language : system
      types    : [python]
```

The [**Ruff**](/integrations/ruff) integration page covers the `pycodestyle` codes to turn off so Ruff's linter does not report the whitespace *Prose* writes.

## Upstream Hook

The [**Ruff pre-commit hook**](https://github.com/astral-sh/ruff-pre-commit) runs Ruff without a system install. Add this block above the `repo: local` block from the Local Hook section:

```yaml-vue
- repo: https://github.com/astral-sh/ruff-pre-commit
  rev: v{{ $frontmatter.ruffVersion }}
  hooks:
    - id: ruff-format
```

`rev:` pins a Ruff release, and the version shown is the one *Prose* builds against, so the two stay in step without a separate pin.

## Exit Codes

The hook reads the same [**Exit Codes**](/reference/exit-codes) the CLI uses. A `format` hook exits 0 once its rewrites are written, so it never fails on a change it made itself, whereas a `check` hook fails the commit whenever a change is pending.
