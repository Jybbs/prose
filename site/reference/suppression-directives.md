# Suppression Directives

Directive shapes opt code out of *Prose*'s rewrites or lints at the file, block, line, and dict-literal scopes. The conceptual narrative for when to reach for suppression lives in the [**Suppression**](/usage/suppression) guide chapter. This page is the canonical index.

Any other directive shape *(`# noqa`, `# pylint: disable`, the wider Python-tooling pragma surface)* is invisible to *Prose*. The walker treats them as ordinary comments and the rules ignore them, so they coexist with the directives below without further wiring.

## Directives

<DirectiveAnatomy />

## Block Markers

`# fmt: off` and `# fmt: on` wrap a region in suppression, with the `# fmt: off` line itself being the directive marker and the following line being the first suppressed line. `# fmt: on` re-enables formatting starting on the next line.

```python
# fmt: off
keep_this_block_exactly_as_written = (1,2,3)
# fmt: on
```

`# prose: off` and `# prose: on` are recognized identically, sharing the same span machinery, so a project can pick whichever prefix reads better in its codebase. `# yapf: disable` and `# yapf: enable` are recognized as block-level equivalents for projects migrating from yapf.

## Line Markers

Line-level directives split by severity, with rewrite suppression taking the `skip` family, lint suppression taking the `ignore` family, and the two independent so a line can carry one of each.

### Rewrite Suppression

`# fmt: skip` *(equivalent to `# prose: skip`)* suppresses every auto-fix rewrite on the line:

```python
data = {"a": 1, "b": 2, "c": 3}  # fmt: skip
```

`# prose: skip` and its bracketed variants narrow to listed rules:

```python
foo = 1  # prose: skip[align-equals]
bar = 2  # prose: skip[align-equals, strip-trailing-commas]
```

A bare `# fmt: skip` or `# prose: skip` widens to every rewrite rule on the line. A bracketed list scopes to the named rules, with unknown rule slugs dropped silently and two bracketed directives on the same line unioning their rule sets.

### Lint Suppression

`# prose: ignore` and its bracketed variants suppress lint diagnostics on the same line:

```python
SCREAMING_CONSTANT = 42  # prose: ignore[reassigned-constants]
TIMEOUT = 30             # prose: ignore[reassigned-constants, single-use-variables]
helper = build_helper()  # prose: ignore
```

A bare `# prose: ignore` suppresses every lint rule on the line. A bracketed list scopes to the named rules.

## Dict-Literal Order Preservation

`# prose: keep` on the opening `{` line of a dict literal tells [[alphabetize]] to leave the entries in their authored order:

```python
config = {  # prose: keep
    "stage_one"   : True,
    "stage_two"   : False,
    "stage_three" : True
}
```

The directive scopes to that one dict literal and doesn't affect any other rule.

## Composition

A single line can carry one block marker, one `# fmt: skip` *(or its `# prose: skip` aliases)*, and one `# prose: ignore[...]` directive. *Prose* parses each independently, so all surfaces compose without ordering constraints. A bare `# prose: ignore` *(no bracket list)* widens any same-line `# prose: ignore[<rule>]` so every lint on the line stays silent, and the same widening applies between bare `# prose: skip` and bracketed `# prose: skip[<rule>]` for rewrites. Two specific bracketed directives of the same family on the same line union their rule slugs:

```python
# fmt: off
data = build()  # prose: ignore[reassigned-constants]  # prose: ignore[single-use-variables]
# fmt: on
```

The same line carries the block marker pair *(opening and closing on the surrounding lines)*, plus two bracketed line directives whose rule lists merge into `{reassigned-constants, single-use-variables}`. A bare `# prose: ignore` anywhere on the line would override both into a widen-to-every-rule.

::: warning Malformed Directives No-Op
A malformed directive *(unclosed brackets, misspelled keyword, trailing text after `ignore`)* parses as a no-op, surfacing nothing and rewriting nothing.
:::

## File-Level Suppression

`# prose: off` on a standalone comment line *(not trailing a statement)* opens a suppression span starting at that line. When no matching `# prose: on` follows, the span runs to EOF, suppressing every *Prose* rewrite below the marker. Placed at the top of the file, the directive consequently covers every line in the file:

```python
# prose: off

# every rule skips this file
def messy(): pass
```

A trailing `# prose: off` on a statement line *(such as `x = 1  # prose: off`)* is ignored, in that the directive must sit on a comment line of its own to register as a span opener. The file-level form is the broadest suppression scope, and for region-bounded suppression the block markers above are the right reach.

## Composition with `--select` / `--ignore`

Per-line and block directives compose against the active rule set. `--select align-equals` narrows the pipeline to one rule, and `# prose: ignore[align-equals]` still suppresses that rule on its line. `--ignore reassigned-constants` drops a rule from the active set, and a line carrying `# prose: ignore[reassigned-constants]` is a no-op since the rule is already not firing.

For the per-rule `enabled` knob, see the [**Configuration**](/reference/configuration) reference. For the conceptual narrative on when to reach for suppression, see the [**Suppression**](/usage/suppression) guide chapter.
