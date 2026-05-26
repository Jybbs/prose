# Suppression

*Prose* is opinionated by design, and most projects benefit from running every rule at its default. Every codebase has its corners, though, and those corners want a way to opt out without dropping a whole rule from the pipeline. The decision is which scope the exception lives at, because *Prose* exposes suppression at four scopes *(file, block, line, dict literal)* and each one fits a different shape of exception.

## Scope Decisions

Each scope carries the directive that fits exceptions at its width. The colored brackets in the gutter trace where each scope binds against a representative source, and the right-side legend gathers every directive against the scope it serves.

<ScopeSpecimen />

## When to Reach for Each

Each suppression directive sits in *Prose*'s [[suppression-map]] index alongside the rule it applies to, and a narrower scope leaves the rest of the file under *Prose*'s defaults. The subsections below pair each scope with the kind of exception it fits, working from broadest to narrowest.

### Disabling a Whole File

`# prose: off` on a comment line near the top of the file short-circuits the whole pipeline to identity before any rule fires. The directive fits generated files, vendored snapshots, or bridging code whose every line carries a constraint a narrower marker would smother. Reach for it only when block-level marker accumulation would itself become noise, because the file-level form opts the file out of every rule *Prose* might add in the future too.

### Bracketing a Block

Block markers fit the case wherein several adjacent lines carry a hand-crafted layout *(a sparse matrix laid out as a 4×4 grid, a state-transition table whose row alignment carries the diagram, an ASCII-art schematic embedded in a comment-fenced region)*. The `# fmt: off` and `# fmt: on` pair brackets the region, leaving every line outside the markers under *Prose*'s defaults. Projects migrating from `yapf` get `# yapf: disable` and `# yapf: enable` as recognized aliases, so the toolchain swap leaves existing markers intact.

```python
# fmt: off
weights = [[0.7, 0.1, 0.1, 0.1],
           [0.1, 0.7, 0.1, 0.1],
           [0.1, 0.1, 0.7, 0.1],
           [0.1, 0.1, 0.1, 0.7]]
# fmt: on
```

### Tagging a Line

Line-level directives split by severity, because rewrites and lints want different escape hatches.

The **`skip`** family covers rewrite suppression. `# fmt: skip` *(equivalent to `# prose: skip`)* exempts the line from every auto-fix rule, fitting cases wherein a single statement carries a deliberate token layout *(a hand-padded dict expression, a one-off argument list whose spacing carries intent)*. `# prose: skip[<rule>]` narrows to the listed rules, so a project that wants only `align-equals` to stay its hand on one line writes `# prose: skip[align-equals]` and leaves the other rewrite rules free to fire.

The **`ignore`** family covers lint suppression. `# prose: ignore[<rule>]` exempts the line from the named lint rules, fitting cases wherein the lint's recommended refactor doesn't apply *(a constant the project genuinely wants pinned at module scope, a single-use variable whose name carries documentation value)*. A bare `# prose: ignore` widens to every lint rule on the line, and the bracketed form scopes precisely.

### Pinning a Dict Literal

`# prose: keep` is the one directive tied to a single rule. [[alphabetize]] reorders dict entries by key as its default, which is the wrong call when source order encodes meaning *(a pipeline whose stages run in declared order, a state machine whose transitions read top-to-bottom as a narrative, a dispatch table whose first match wins)*. The marker on the opening `{` line tells [[alphabetize]] to leave that one literal's authored order alone, and no other rule notices the directive.

```python
stages = {  # prose: keep
    "fetch"    : fetch_payload,
    "parse"    : parse_records,
    "validate" : validate_schema,
    "render"   : render_html
}
```

## See Also

For the exact directive syntax, alias surfaces, malformed-directive behavior, and composition rules, see the [**Suppression Directives**](/reference/suppression-directives) reference. For the per-rule `enabled` toggle that disables a rule at the configuration level rather than per-scope, see the [**Configuration**](/reference/configuration) reference.
