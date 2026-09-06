# Suppression

*Prose* is opinionated by design, and most projects run every rule at its default. A suppression directive exempts one place from a rule without turning that rule off for the whole project. *Prose* offers suppression at the file, block, line, and dict-literal scopes, and choosing a directive means choosing the narrowest scope that covers the exception.

## Scope Decisions

Each directive covers exceptions at one scope, and the colored brackets in the gutter mark where each scope applies in a sample file, with the legend on the right listing every directive under the scope it covers.

<ScopeSpecimen />

## When to Reach for Each

The subsections below go from the broadest scope to the narrowest, because a narrower scope leaves the rest of the file under the defaults. Whichever scope a directive takes, it is recorded in the file's [[suppression-map]], and each rule checks that map before writing an edit or reporting a finding.

### Disabling a Whole File

`# prose: off` on its own comment line near the top of a file turns every rule off for that file, before any rule runs. It fits generated files, vendored snapshots, and bridging code where nearly every line has a reason to stay as written. Reach for it only when block markers would pile up, because the file-level directive also exempts the file from every rule a later release adds.

### Bracketing a Block

Block markers fit several adjacent lines that carry a hand-made layout *(a sparse matrix written as a 4×4 grid, a state-transition table whose row alignment is the diagram, ASCII art inside a comment)*. `# fmt: off` and `# fmt: on` enclose the region, and every line outside them stays under the defaults. `# yapf: disable` and `# yapf: enable` work as aliases, so a project moving from `yapf` keeps its existing markers.

```python
# fmt: off
weights = [[0.7, 0.1, 0.1, 0.1],
           [0.1, 0.7, 0.1, 0.1],
           [0.1, 0.1, 0.7, 0.1],
           [0.1, 0.1, 0.1, 0.7]]
# fmt: on
```

The markers exempt only the lines between them, so [[alphabetize-siblings]] still reorders the module-level assignments above and below the bracket, and the bracketed region itself stays exactly as written.

### Tagging a Line

Line-level directives come in two families, one for rewrites and one for lints, because the two need different escapes.

The **`skip`** family exempts a line from rewrites, where `# fmt: skip` *(or its equivalent `# prose: skip`)* at the end of a statement exempts the whole logical line from every auto-fix rule, so a statement spanning several physical lines is exempt from its first line through the line carrying the directive. It fits a statement whose spacing is deliberate *(a hand-padded dict, a one-off argument list laid out to read a certain way)*. `# prose: skip[<rule>]` narrows the exemption to the named rules, where `# prose: skip[align-equals]` exempts one statement from `align-equals` and leaves every other rewrite rule free to run. When the exempted statement is a single line inside an alignment group, the other rows still align around it, so the exempt row reads as a deliberate exception rather than breaking the group.

The **`ignore`** family silences lints, where `# prose: ignore[<rule>]` at the end of a line silences the named lint rules on it, for a case where the lint's suggested change does not apply *(a constant the project reassigns on purpose, a binding whose name explains a value the inlined expression would leave unnamed)*. A bare `# prose: ignore` silences every lint on the line.

The two families stay separate, so a statement that needs both its layout kept and its lint silenced carries one of each. Only the block markers cover both at once, since a `# fmt: off` region suppresses rewrites and lint diagnostics together for every line it encloses.

### Pinning a Dict Literal

`# prose: keep` on the opening `{` line or the closing `}` line of a dict literal keeps that one literal's order as written, and it is the one directive tied to a single construct. The default it overrides is [[alphabetize-siblings]] sorting dict entries by key, which is wrong where the source order carries meaning *(a pipeline whose stages run in the order written, a state machine whose transitions read top to bottom, a dispatch table where the first match wins)*. [[band-constants]] reads the same marker and leaves the statement where the author put it rather than gathering it into the band. Where a whole project reads its dicts in order, the `sort-dict-keys` facet turns the sort off everywhere, leaving the directive for the remaining exceptions. The same marker on an `__all__` or `__slots__` list keeps that one hand-ordered list, and the `sort-dunder-lists` facet is its project-wide counterpart.

```python
stages = {  # prose: keep
    "fetch"    : fetch_payload,
    "parse"    : parse_records,
    "validate" : validate_schema,
    "render"   : render_html
}
```

## See Also

The [**Suppression Directives**](/reference/suppression-directives) reference gives the exact syntax of every directive, its aliases, what happens to a malformed directive, and how directives combine. The [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` facet, which turns a rule off for the whole project rather than for one place.
