# Suppression Directives

Five directive shapes opt code out of *Prose*'s rewrites or lints at three scopes *(file, block, line)*. The conceptual narrative for when to reach for suppression lives in the [**Suppression**](/guide/suppression) guide chapter. This page is the lookup table.

## Directive Table

| Directive | Scope | Effect |
|---|---|---|
| `# fmt: off` / `# fmt: on` | Block | Every auto-fix rule honors the markers, so a hand-tuned region survives the formatter pass intact |
| `# yapf: disable` / `# yapf: enable` | Block | Aliases for `# fmt: off` / `# fmt: on`. Recognized to ease migration from yapf |
| `# fmt: skip` | Line | Auto-fix rules skip the line carrying the directive |
| `# prose: ignore` | Line | All lint rules skip the line. Pairs with `[<rule_id>]` to narrow |
| `# prose: ignore[<rule>, <rule>]` | Line | Listed lint rules skip the line |
| `# prose: keep` | Block *(dict literal)* | [[alphabetize]] leaves the dict entries in their authored order |

## Block Markers

`# fmt: off` and `# fmt: on` wrap a region in suppression, with the `# fmt: off` line itself being the directive marker and the following line being the first suppressed line. `# fmt: on` re-enables formatting starting on the next line.

```python
# fmt: off
keep_this_block_exactly_as_written = (1,2,3)
# fmt: on
```

`# yapf: disable` and `# yapf: enable` are recognized as block-level equivalents for projects migrating from yapf.

## Line Markers

`# fmt: skip` applies to the line carrying the directive only:

```python
data = {"a": 1, "b": 2, "c": 3}  # fmt: skip
```

`# prose: ignore` and its bracketed variants suppress lint diagnostics on the same line:

```python
SCREAMING_CONSTANT = 42  # prose: ignore[loose-constants]
TIMEOUT = 30             # prose: ignore[loose-constants, single-use-variables]
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

A single line can carry one block marker, one `# fmt: skip`, and one `# prose: ignore[...]` directive. *Prose* parses each independently, so all three surfaces compose without ordering constraints. A bare `# prose: ignore` *(no bracket list)* widens any same-line `# prose: ignore[<rule>]` so every lint on the line stays silent. Two specific bracketed directives on the same line union their rule ids. Malformed directives *(unclosed brackets, misspelled keyword, trailing text after `ignore`)* parse as no-ops, surfacing nothing and rewriting nothing.

## File-Level Suppression

A file-level `# prose: off` directive on a comment line near the top of the file suppresses every *Prose* rewrite for the entire file. The pipeline detects the directive in the [[suppression-map]] built during [[source]] construction and short-circuits to identity before any rule fires.

```python
# prose: off

# every rule skips this file
def messy(): pass
```

The file-level directive is the broadest suppression scope. For region-bounded suppression, use the block markers above.

## Composition with `--select` / `--ignore`

Per-line and block directives compose against the active rule set. `--select align-equals` narrows the pipeline to one rule, and `# prose: ignore[align-equals]` still suppresses that rule on its line. `--ignore loose-constants` drops a rule from the active set, and a line carrying `# prose: ignore[loose-constants]` is a no-op since the rule is already not firing.

For the per-rule `enabled` knob, see the [**Configuration**](/reference/configuration) reference. For the conceptual narrative on when to reach for suppression, see the [**Suppression**](/guide/suppression) guide chapter.
