---
description: "Covers what each exit code means, which is the contract a CI gate reads."
---

# Exit Codes

Every `prose check` and `prose format` run exits with one of the codes below, which is the contract a CI gate reads. When two outcomes apply to one run, the higher code wins, except that a stdin `prose format` writing the formatted source exits `0` whatever lint it found.

::: info Per-File Failures Stay Local
A parse failure in one file produces exit code `3` for that file, and every other file in the tree is still formatted or checked.
:::

<ExitCodeMatrix />

## CI Gating

A gate that treats rewrites and lints apart can branch on `2` independently of `1`, because a lint finding never auto-fixes and so never resolves on its own. A gate over a stdin buffer, such as a hook reading the staged content of each file, runs `prose check -`, since a stdin `prose format` exits `0` once it writes the formatted source. The [**GitHub Actions**](/integrations/github-actions) integration page covers the CI job, its Ruff step, and the formats that show findings beside the gate.

## Help Output

`prose --help` prints the same table below its option list, so the codes are readable from the terminal.

The [**CLI Reference**](/reference/cli) lists the flags that produce each code.
