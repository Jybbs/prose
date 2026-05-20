# Pre-Commit

<Tool slug="precommit" /> wires *Prose* into the git commit boundary, so a staged change either matches the canonical layout or the commit fails. The hook runs against staged files only, which keeps the loop tight on edited code without re-walking the whole tree.

## Local Hook

Add a `local` hook to your `.pre-commit-config.yaml`:

```yaml
- repo: local
  hooks:
    - id      : prose
      name    : prose
      entry   : prose format
      language: system
      types   : [python]
```

`entry: prose format` rewrites the staged file in place, so the hook surfaces as a *"fixed by **Prose**"* diff the developer re-stages and recommits. The rewrite lands on disk, leaving the working tree dirty with respect to the index, which means a `git add` of the affected files and a fresh `git commit` to bring the rewrites into the staging area. Swap `entry: prose check` for the check-only variant, which fails the commit when changes are pending without writing to disk.

The `language: system` setting tells pre-commit to use the `prose` binary already installed on the developer's `PATH` *(per the [**Installation**](/guide/installation) chapter)* rather than installing a pinned version into the hook's sandbox. Projects that prefer a sandboxed pin can use `language: python` with `additional_dependencies: ["prose-formatter==<version>"]` instead.

## Two-Stage Pipeline

For the canonical `ruff format && prose format` ordering, chain two hooks in the same `repo: local` block so Ruff settles tokens before *Prose* lays out lines:

```yaml
- repo: local
  hooks:
    - id      : ruff-format
      name    : ruff format
      entry   : ruff format
      language: system
      types   : [python]
    - id      : prose
      name    : prose
      entry   : prose format
      language: system
      types   : [python]
```

pre-commit runs hooks in declaration order against the staged file set, so Ruff's rewrite settles first and *Prose* picks up the settled token surface. The [**Ruff**](/integrations/ruff) integration page covers the `extend-ignore` configuration that keeps Ruff's `pycodestyle` lints quiet on the whitespace *Prose* introduces.

## Upstream Hook

The [**`pre-commit`-managed Ruff hook**](https://github.com/astral-sh/ruff-pre-commit) handles the Ruff side without requiring a system install. Pair it with the *Prose* local hook above:

```yaml
- repo: https://github.com/astral-sh/ruff-pre-commit
  rev: v0.15.10
  hooks:
    - id: ruff-format

- repo: local
  hooks:
    - id      : prose
      name    : prose
      entry   : prose format
      language: system
      types   : [python]
```

The `rev:` field pins to a specific Ruff release. The version family is the same one *Prose* compiles against on the Astral side, so the two stay in lockstep without a custom pin matrix.

## Exit Codes

The hook surfaces the same [**Exit Codes**](/reference/exit-codes) the CLI uses, so a `format` hook never fails on rewrites it applies *(those resolve to exit 0 once the rewrite lands)* and a `check` hook fails the commit when changes are pending.
