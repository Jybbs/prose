---
description: "Walks through choosing a directive for each scope, from a whole file down to one dict or class body."
---

# Suppression

*Prose* is opinionated by design, and most projects run every rule at its default. A suppression directive exempts one place from a rule without turning that rule off for the whole project. *Prose* offers suppression at the file, block, line, and construct scopes, and choosing a directive means choosing the narrowest scope that covers the exception. This chapter walks through that choice, whereas the [**Suppression Directives**](/reference/suppression-directives) reference gives the exact syntax of every directive and alias, what a malformed directive does, and how directives combine.

## Scope Decisions

Each directive covers exceptions at one scope, and the colored brackets in the gutter mark where each scope applies in a sample file, with the legend on the right listing every directive under the scope it covers.

<ScopeSpecimen />

## Choosing a Directive

The subsections below go from the broadest scope to the narrowest, because a narrower scope leaves the rest of the file under the defaults. Every directive but `# prose: keep` is recorded in the file's [[suppression-map]], which each rule checks before writing an edit or reporting a finding, whereas `# prose: keep` is read off the construct it marks.

### Disabling a Whole File

`# prose: off` on its own comment line near the top of a file turns every rule off for that file. It fits generated files, vendored snapshots, and bridging code where nearly every line has a reason to stay as written. Reach for it only when block markers would pile up, because the file-level directive also exempts the file from every rule a later release adds. The [**File-Level Suppression**](/reference/suppression-directives#file-level-suppression) reference covers where the directive has to sit.

### Bracketing a Block

Block markers fit several adjacent lines that carry a hand-made layout *(a sparse matrix written as a 4×4 grid, a state-transition table whose row alignment is the diagram, ASCII art inside a comment)*. `# fmt: off` and `# fmt: on` enclose the region, and every line outside them stays under the defaults, so the rules still reach the code on either side.

```python
# fmt: off
weights = [[0.7, 0.1, 0.1, 0.1],
           [0.1, 0.7, 0.1, 0.1],
           [0.1, 0.1, 0.7, 0.1],
           [0.1, 0.1, 0.1, 0.7]]
# fmt: on
```

A region holds out lint findings as well as rewrites, so it suits a stretch of code that every rule should leave alone. The [**Block Markers**](/reference/suppression-directives#block-markers) reference lists the `# prose:` and `# yapf:` spellings of the same pair.

### Tagging a Line

A line-level directive fits one statement, and its family follows from what the exception holds out, because a rewrite and a lint each need their own escape:

- A statement whose layout is deliberate (*a hand-padded dict, a one-off argument list laid out to read a certain way*) takes a `skip`. `# fmt: skip` at its end keeps the statement as written under every rewriting rule, whereas `# prose: skip[<rule>]` keeps it from the rules it names and leaves every other rewrite free to run
- A lint finding whose suggested change does not apply (*a constant the project reassigns on purpose, a binding whose name explains a value the inlined expression would leave unnamed*) takes an `ignore`. `# prose: ignore[<rule>]` at the end of the line silences the lint rules it names there

A statement that needs both its layout kept and its lint silenced carries one of each, since a `skip` reaches no lint and an `ignore` no rewrite. [[inert-directives]] reports a `skip` or `ignore` that holds nothing out, so a directive written in the wrong family, or left behind once the code it covered changed, shows up under `prose check`. The [**Line Markers**](/reference/suppression-directives#line-markers) reference gives every form, the lines a directive covers on a wrapped statement, and what a skip does to an alignment group and to the line a layout rule would have split.

### Pinning a Dict or a Class Body

`# prose: keep` is the one directive tied to a single construct. It holds a dict literal's entries in the order written, for a dict whose order carries meaning *(a pipeline whose stages run in the order written, a state machine whose transitions read top to bottom, a dispatch table where the first match wins)*, and it does the same for an `__all__` or `__slots__` list.

```python
stages = {  # prose: keep
    "fetch"    : fetch_payload,
    "parse"    : parse_records,
    "validate" : validate_schema,
    "render"   : render_html
}
```

On a `class` line, the marker holds the statements of the class body as written, which is the escape for a class whose field order carries meaning *(a `pandera.DataFrameModel` whose columns follow the declaration order, a form whose fields render top to bottom)*.

```python
class StationSchema(DataFrameModel):  # prose: keep
    station  : Index[str] = Column(check_name=True)
    name     : str        = Column()
    latitude : float      = Column()
```

Where a whole project reads its dicts in order, the `sort-dict-keys` facet of [[alphabetize-siblings]] turns the sort off everywhere, and `sort-dunder-lists` does the same for every `__all__` and `__slots__` list, each leaving the directive for the exceptions that remain. The [**Construct Order Preservation**](/reference/suppression-directives#construct-order-preservation) reference lists every order the marker holds and everything inside the construct that still sorts.

## See Also

The [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` facet, which turns a rule off for the whole project rather than for one place.
