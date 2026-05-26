# Pre-Commit

<Tool slug="precommit" /> wires *Prose* into the git commit boundary, so a staged change either matches the canonical layout or the commit fails. The hook runs against staged files only, which keeps the loop tight on edited code without re-walking the whole tree.

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

`entry: prose format` rewrites the staged file in place, so the hook surfaces as a *"fixed by **Prose**"* diff that fails the commit and leaves the rewrite on disk for the developer to re-stage and re-commit. This is the loop pre-commit uses for every fixer-style hook *(Black, isort, autopep8 behave the same way)*. For a check-only variant that fails on pending rewrites without writing to disk, swap `entry: prose check`, leaving the developer to run `prose format` manually before retrying the commit.

The `language: system` setting tells pre-commit to use the `prose` binary already installed on the developer's `PATH` *(per the [**Installation**](/usage/installation) chapter)* rather than installing a pinned version into the hook's sandbox. Projects that prefer a sandboxed pin can use `language: python` with `additional_dependencies: ["prose-formatter==<version>"]` instead.

## Pairing With Ruff

For projects that pair *Prose* with `ruff format`, add a Ruff hook above the *Prose* hook in the same `repo: local` block. pre-commit runs hooks in declaration order, so Ruff settles tokens before *Prose* lays out lines. The added hook reads:

```yaml
    - id       : ruff-format
      name     : ruff format
      entry    : ruff format
      language : system
      types    : [python]
```

The [**Ruff**](/integrations/ruff) integration page covers the `extend-ignore` configuration that keeps Ruff's `pycodestyle` lints quiet on the whitespace *Prose* introduces.

## Upstream Hook

The [**`pre-commit`-managed Ruff hook**](https://github.com/astral-sh/ruff-pre-commit) handles the Ruff side without requiring a system install. Add this block above the `repo: local` from the Local Hook section:

```yaml
- repo: https://github.com/astral-sh/ruff-pre-commit
  rev: v0.15.10
  hooks:
    - id: ruff-format
```

The `rev:` field pins to a specific Ruff release. The version family is the same one *Prose* compiles against on the Astral side, so the two stay in lockstep without a custom pin matrix.

## Exit Codes

The hook surfaces the same [**Exit Codes**](/reference/exit-codes) the CLI uses, so a `format` hook never fails on rewrites it applies *(those resolve to exit 0 once the rewrite lands)* and a `check` hook fails the commit when changes are pending.
