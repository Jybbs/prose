# Suppression

*Prose* is opinionated by design, and most projects benefit from running every rule at its default. Every codebase has its corners, though, and those corners want a way to opt out without dropping a whole rule. *Prose* exposes three suppression surfaces (*block markers, line markers, and lint directives*) so the opt-out fits the scope of the exception.

## Block Markers

Wrap any region in `# fmt: off` / `# fmt: on` to keep its layout exactly as written. Every rewriting rule honors the markers, so a hand-tuned data table or an embedded ASCII diagram survives the formatter pass intact.

```python
# fmt: off
keep_this_block_exactly_as_written = (1,2,3)
# fmt: on
```

The block boundary is the `# fmt: off` line itself, with the following line being the first suppressed line. `# fmt: on` re-enables formatting starting on the next line.

The aliases `# yapf: disable` and `# yapf: enable` are recognized as block-level equivalents, letting projects migrating from yapf preserve their existing suppression spans.

## Line Markers

`# fmt: skip` on a single source line opts that statement out without surrounding markers, which is the right shape when only one line wants the exemption:

```python
data = {"a": 1, "b": 2, "c": 3}  # fmt: skip
```

## Lint Directives

Lint diagnostics (*[**`legacy-union-syntax`**](/rules/legacy-union-syntax), [**`loose-constants`**](/rules/loose-constants), [**`no-step-narration`**](/rules/no-step-narration), [**`single-use-variables`**](/rules/single-use-variables)*) opt out per line through `# prose: ignore[<rule>]`:

```python
SCREAMING_CONSTANT = 42  # prose: ignore[loose-constants]
```

A bare `# prose: ignore` suppresses every lint rule on the line, and `# prose: ignore[a, b]` lists several:

```python
TIMEOUT = 30  # prose: ignore[loose-constants, single-use-variables]
```

The directive applies only to the line carrying the comment, so to suppress an entire range, reach for `# fmt: off` / `# fmt: on` instead.

## Preserve Source Order

Inside a dictionary literal, a `# prose: keep` comment on the opening `{` line tells [**`alphabetize`**](/rules/alphabetize) to leave the entries in their authored order:

```python
config = {  # prose: keep
    "stage_one"   : True,
    "stage_two"   : False,
    "stage_three" : True
}
```

The directive is scoped to that one dict and doesn't suppress any other rule. Useful when the entry order carries meaning a future reader needs preserved, like a pipeline-stage sequence or a column layout.

## Composition

A single line can carry one block marker, one `# fmt: skip`, and one `# prose: ignore[...]` directive. *Prose* parses each independently, so all three surfaces compose without ordering constraints. A bare `# prose: ignore` (*no bracket list*) widens any same-line `# prose: ignore[<rule>]` so every lint on the line stays silent. Two specific directives on the same line union their rule ids. Malformed directives (*unclosed brackets, misspelled keyword, trailing text after `ignore`*) parse as no-ops, surfacing nothing and rewriting nothing.

For the underlying machinery, the [**`SuppressionMap`**](/primitives/suppression-map) primitive walks every source file once during [**`Source`**](/primitives/source) construction and indexes all four directive shapes. The [**`Pipeline`**](/primitives/pipeline) consults the map at the edit-emission boundary, dropping suppressed entries before they surface.
