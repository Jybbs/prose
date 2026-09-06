---
description: "Covers what each exit code means, which is the contract a CI gate reads."
---

# Exit Codes

Every `prose check` and `prose format` run exits with one of the codes below, which is the contract a CI gate reads. When two outcomes apply to one run, the higher code wins. A `format` run that writes an auto-fix exits `0` once the rewrite is written, since the change was applied rather than left pending, whereas `check` on the same source exits `1`.

::: info Per-File Failures Stay Local
A parse failure in one file produces exit code `3` for that file, and every other file in the tree is still formatted or checked.
:::

<ExitCodeMatrix />

## CI Gating

The usual CI job is one `prose check` step that fails on any non-zero exit. A project that wants to gate rewrites and lints separately can branch on `2` independently of `1`, because a lint finding never auto-fixes and so never resolves on its own.

```yaml
- run: uv tool install prose-formatter
- run: prose check .
```

The [**GitHub Actions**](/integrations/github-actions) integration page covers the annotation and SARIF formats that show findings inline beside the gate.

## Composition With Ruff

When a job [**runs Ruff too**](/integrations/ruff), each tool exits with its own code from its own step, and a non-zero exit from either fails the gate with no extra wiring. The codes do not combine, so the failing step tells the developer which tool reported the finding. Under the default step order the job stops at the first non-zero step, so when both would fail, only the first shows. `if: always()` on the *Prose* step runs it regardless, so both results appear in one run.

## Help Output

`prose check --help` and `prose format --help` print the same table below the option list, so the codes are readable from the terminal.

The [**CLI Reference**](/reference/cli) lists the flags that produce each code.
