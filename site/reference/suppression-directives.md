---
description: "Covers `# fmt: off / on`, `# fmt: skip`, the `# yapf` aliases, `# prose: ignore`, and `# prose: keep`."
---

# Suppression Directives

A directive exempts code from *Prose*'s rewrites or lints at the file, block, line, or dict-literal scope. The [**Suppression**](/usage/suppression) chapter covers when to reach for each, and this page is the complete list.

A pragma from another tool *(`# pylint: disable`, `# type: ignore`, `# pyright: ignore`, and the rest of the Python tooling pragmas)* is invisible to *Prose* wherever it is not named below. The walker reads it as an ordinary comment and the rules ignore it, so it sits beside the directives further down with no further setup.

The foreign comments a rule does read are the ones stating an intent no static read of the file reaches, and each suppresses nothing even so. [[prune-inert-imports]] reads a bare `# noqa`, or one naming `F401`, trailing an import as a marker that the statement re-exports what it binds, in the same way it reads an `__all__` entry and the PEP 484 `from x import y as y` form. It reads a file-level `# ruff: noqa: F401`, `# flake8: noqa: F401`, or `# pyright: reportUnusedImport=false` opening a line of its own at column zero the same way, holding every unreferenced import in the module rather than one statement's. Each head is matched as its own tool matches it, so the spacing is free, `flake8` is read in any casing whereas `ruff` and `pyright` are read only in lower case, a pyright rule reports nothing on either `false` or `none` in any casing while a severity such as `error` leaves it reporting, and an indented pragma sits inside a block and holds nothing. A head naming no code is not read, since silencing every rule a tool carries says nothing about re-exports in particular. A marker counts only where it opens a comment rather than where the word appears in prose. [[band-constants]] reads a `# noqa` naming `E402` as a marker that the import stays on the line its author gave it. Those are the whole of the set, and none of them exempts anything from a rewrite or a lint.

## Directives

<DirectiveAnatomy />

## Block Markers

`# fmt: off` and `# fmt: on` enclose a region. The `# fmt: off` line is the marker and the next line is the first exempt line, and `# fmt: on` turns formatting back on from the line after it.

```python
# fmt: off
keep_this_block_exactly_as_written = (1,2,3)
# fmt: on
```

`# prose: off` and `# prose: on` work identically and share the same machinery, so a project can use whichever prefix reads better. `# yapf: disable` and `# yapf: enable` are block markers too, for a project moving from yapf.

## Line Markers

Line-level directives come in two families, `skip` for rewrites and `ignore` for lints, and the two are independent so a line can carry one of each.

### Rewrite Suppression

`# fmt: skip` *(or its equivalent `# prose: skip`)* exempts the logical line it ends from every auto-fix rewrite:

```python
data = {"a": 1, "b": 2, "c": 3}  # fmt: skip
```

A directive at the end of a wrapped statement covers every physical line the statement spans, from its first line through the directive's own, so a rule's edits to that statement are withheld together rather than applied to part of it:

```python
z = (
    x
)  # fmt: skip
```

A directive inside a bracketed construct, where the logical line continues past it, covers only its own physical line.

The bracketed forms of `# prose: skip` narrow the exemption to the listed rules:

```python
foo = 1  # prose: skip[align-equals]
bar = 2  # prose: skip[align-equals, strip-trailing-commas]
```

A bare `# fmt: skip` or `# prose: skip` exempts the logical line from every rewrite rule. A bracketed list names the rules, an unknown slug in the list is ignored, and two bracketed directives on one line combine their lists.

A skip reaches the rewrite rules only, so a lint finding on the statement still prints, and silencing it takes a `# prose: ignore` beside the skip. The block markers above differ, since a `# fmt: off` region suppresses rewrites and lint findings together.

### Lint Suppression

`# prose: ignore` and its bracketed forms silence lint findings on the same line:

```python
SCREAMING_CONSTANT = 42  # prose: ignore[reassigned-constants]
TIMEOUT = 30             # prose: ignore[reassigned-constants, inlinable-bindings]
helper = build_helper()  # prose: ignore
```

A bare `# prose: ignore` silences every lint rule on the line. A bracketed list names the rules.

## Dict-Literal Order Preservation

`# prose: keep` on the opening `{` line or the closing `}` line of a dict literal keeps the entries in the order written, so [[alphabetize-siblings]] leaves them alone:

```python
config = {  # prose: keep
    "stage_one"   : True,
    "stage_two"   : False,
    "stage_three" : True
}
```

The directive covers that one literal, where [[alphabetize-siblings]] keeps the entry order and [[band-constants]] leaves the statement out of the band, and the same marker on an `__all__` or `__slots__` list keeps that one list as written.

## Composition

One line can carry a block marker, `# fmt: skip` or `# prose: skip` directives, and `# prose: ignore[...]` directives together. *Prose* parses each on its own, so they combine in any order. A bare `# prose: ignore` *(no bracket list)* widens any `# prose: ignore[<rule>]` on the same line so every lint on the line is silenced, and a bare `# prose: skip` widens a bracketed `# prose: skip[<rule>]` the same way for rewrites. Two bracketed directives of the same family on one line combine their slugs:

```python
# fmt: off
data = build()  # prose: ignore[reassigned-constants]  # prose: ignore[inlinable-bindings]
# fmt: on
```

The line above carries the block marker pair *(opening and closing on the surrounding lines)* plus two bracketed line directives whose lists combine into `{reassigned-constants, inlinable-bindings}`. A bare `# prose: ignore` anywhere on the line would widen both to every rule.

::: warning Malformed Directives No-Op
A malformed directive *(unclosed brackets, a misspelled keyword, trailing text after `ignore`)* is read as no directive, so it reports nothing and rewrites nothing.
:::

## File-Level Suppression

`# prose: off` on a comment line of its own *(not at the end of a statement)* opens an exempt region at that line. With no `# prose: on` after it, the region runs to the end of the file, exempting every line below the marker from every rewrite, whereas inside a notebook it closes at the end of its own code cell rather than reaching the next. At the top of a file the directive therefore covers the whole file:

```python
# prose: off

# every rule skips this file
def messy(): pass
```

A `# prose: off` at the end of a statement *(such as `x = 1  # prose: off`)* is ignored, since the directive opens a region only from a comment line of its own. The file-level form is the widest scope, and a bounded region takes the block markers above.

## Composition With `--select` / `--ignore`

Line and block directives apply on top of the active rule set. `--select align-equals` narrows the run to one rule, and `# prose: skip[align-equals]` still exempts its line from that rule. `--ignore reassigned-constants` removes a rule from the run, so a line carrying `# prose: ignore[reassigned-constants]` changes nothing, since the rule was not going to report.

The [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` facet, and the [**Suppression**](/usage/suppression) chapter covers when to reach for each directive.
