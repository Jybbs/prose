# Configuration

*Prose* loads its configuration from a `prose.toml` file or the `[tool.prose]` table of a `pyproject.toml`, walking upward from each input file's directory to the nearest one. With no configuration, every rule runs at its default, in that a project that writes no config gets the canonical *Prose* shape automatically.

A `prose.toml` keeps its keys at the document root, the form this page shows throughout. A `pyproject.toml` carries the same keys under a `[tool.prose]` prefix so the manifest can house other tools too, leaving every key below a `[tool.prose.<…>]` equivalent for projects that prefer one file.

`target-version` carries the bare `major.minor` form *(`"3.13"`, `"3.14"`)* used by `mypy`'s `python_version` setting, with rules whose safety depends on the runtime reading the field directly. The docstring-budget duality *(`code-line-length` for Title-case-headed structured sections, `docstring-line-length` for description prose)* lets a project keep code-shaped tables wide while keeping description prose at a comfortable reading measure, and `docstring-structured-policy` collapses both to a single budget when a project prefers a uniform width.

To turn rules off or tune them, write the `[rules]` table:

```toml
code-line-length = 88

[rules]
align-colons      = { max-shift = false }
align-equals      = false
collection-layout = { max-atomics-per-line = 3 }
```

A bare `false` disables a rule, an inline table sets its facets while leaving the rule enabled, and a rule you do not name stays on at its default. Under `pyproject.toml` the table reads `[tool.prose.rules]`, and a rule with several facets may prefer the expanded `[rules.<rule>]` sub-table *(`[tool.prose.rules.<rule>]` in the manifest)*, which carries the same settings as the inline form.

## Where *Prose* Looks

*Prose* walks upward from each input file's own directory toward the filesystem root, so a file answers to its own project's config even when one invocation names files across several projects. Stdin input walks from the working directory, the one input with no path of its own. In each directory a `prose.toml` outranks a `pyproject.toml`, and the nearest directory carrying either wins, in that *Prose* reads only that one file and never merges across matches up the tree. A `pyproject.toml` lacking a `[tool.prose]` table is passed over, leaving the walk to continue upward. A standalone script the walk never resolves to a project reads its own `[tool.prose]` from a leading PEP 723 `# /// script` block, the one configuration home a single-file script has, whereas a script under a project ignores its block and answers to the project. When neither an ancestor nor a block carries config, every default applies as if the config were empty.

When a `prose.toml` and a `pyproject.toml` `[tool.prose]` table share a directory, the `prose.toml` wins and *Prose* notes the precedence to stderr, so the file that took effect is never ambiguous.

## Top-Level Keys

The top-level keys carry settings that span multiple rules. They sit at the document root in a `prose.toml` and under `[tool.prose]` in a `pyproject.toml`.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `code-line-length` | positive int | `88` | Honored by line-length-aware rules |
| `docstring-line-length` | positive int | `76` | Description-prose budget for [[docstring-wrap]] |
| `docstring-structured-policy` | `"code-line-length"` \| `"docstring-line-length"` | `"code-line-length"` | Source budget for structured docstring sections |
| `import-line-length` | positive int \| `false` | `120` | Import-wrap budget for [[import-layout]], falling back to `code-line-length` when `false` |
| `target-version` | `"3.X"` version string | unset | Python runtime the project ships to, consumed by version-gated rules |

`target-version` names the Python runtime a project ships to, taking the bare `major.minor` form (*`"3.13"`, `"3.14"`*) used by `mypy`'s `python_version` setting. Rules whose safety depends on the runtime read this field directly. [[legacy-union-syntax]] and [[unused-future-annotations]] are the two current consumers.

::: info Version Gates Need Opt-In
With no value set, every version-dependent arm skips rather than assume a default, leaving [[legacy-union-syntax]] and [[unused-future-annotations]] quiet on every project that has not opted into a target.
:::

## Cache

The `[cache]` table tunes the user-level [**cache**](/reference/cache) that *Prose* keeps for repeat runs *(`[tool.prose.cache]` in a `pyproject.toml`)*. Both keys default to the canonical shape, so a project that does not write the table gets the cache at its full size.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the cache globally |
| `max-size-mib` | positive int | `100` | LRU eviction cap on the cache directory |

```toml
[cache]
enabled      = true
max-size-mib = 250
```

## Imports

The `[imports]` table names the project's first-party packages *(`[tool.prose.imports]` in a `pyproject.toml`)*, so [[alphabetize]] groups their imports with relative imports in the local-package block rather than the external `from` block. With no list, only relative imports (`from .`, `from ..pkg`) populate the local-package group.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `first-party` | list of package names | `[]` | Root package names whose imports lift into the local-package group |

```toml
[imports]
first-party = ["myapp", "acme"]
```

A list entry names a root package, so `myapp` matches `import myapp.db` and `from myapp import app` while leaving `from myapplication import x` in the external `from` group.

## Per-Rule Facets

The `[rules]` table holds one entry per rule you change. A bare bool is the shorthand for `enabled` (*`alphabetize = false`*), an inline table sets a rule's facets (*`align-equals = { max-shift = 4 }`*), and a rule you do not name stays enabled at its defaults. The table below lists the facets each rule accepts, with the *Where* column resolving which rules share a facet. Generic facets *(`enabled`, `max-shift`)* apply to every rule in their column's named category. Rule-specific facets read inputs scoped to the named rule, even when two rules happen to spell their facet the same way *(the two `allow` rows are distinct, scoped to different rules and reading different inputs)*.

| Key | Type | Where | Default | Meaning |
|---|---|---|---|---|
| `enabled` | bool | every rule | `true` | Toggle the rule on or off |
| `max-shift` | positive int \| `0` \| `false` | alignment rules | `16` | Width-spread budget for an alignment run. A positive `N` caps the spread, `0` forbids any shift so every row sits flush, and `false` lifts the cap so a contiguous run folds into one column. To leave one row out of an otherwise-aligned group, hold it with `# prose: skip` |
| `sort-docstring-entries` | bool | [[alphabetize]] | `true` | Reorder `name: description` entries within Title-case-headed docstring sections, parameter entries mirroring the signature as the rule leaves it and stragglers alphabetizing below. Set `false` to keep narrative-curated entry order while still sorting every other surface |
| `collapse` | bool | [[collection-layout]] | `true` | Join a fitting multi-line literal, subscript, or dict key back to one line. `false` freezes those shapes where they sit |
| `explode` | bool | [[collection-layout]] | `true` | Expand an overflowing or over-count collection to one entry per line. `false` suppresses every expansion and leaves the count cap inert |
| `max-atomics-per-line` | positive int \| `false` | [[collection-layout]] | `8` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap. `false` removes the cap and packs by width alone |
| `max-inline-dict-entries` | positive int \| `false` | [[collection-layout]] | `3` | Expand a dict once its entry count exceeds the cap, whatever its width. `false` disables the count trigger |
| `wrap-dict-entries` | bool | [[collection-layout]] | `true` | Break an over-wide `key: value` at its `:` and hang the value beneath. `false` leaves the oversized entry on one line |
| `max-inline-args` | positive int \| `false` | [[call-layout]] | `3` | Explode a call to one keyword argument per line once its argument count exceeds the cap. `false` disables the count trigger and leaves every call inline |
| `allow` | list of module names | [[bare-imports]] | `[]` | Modules whose bare-import form is preserved whatever their attribute count |
| `exempt-aliased` | bool | [[bare-imports]] | `true` | Exempt every aliased bare import (*`import x as y`*) from the rule |
| `max-attributes` | positive int | [[bare-imports]] | `4` | Distinct-attribute count at or below which an unaliased bare import is flagged |
| `allow` | list of names | [[reassigned-constants]] | `[]` | Module-level names exempted from the lint |
| `allow-pattern` | regex | [[single-use-variables]] | `"^_"` | Binding names exempted from the lint |

## Rule Categories

Rules sit in configuration buckets, with each bucket carrying a distinct facet shape.

### Alignment Rules

The rules that line columns vertically share one structural question, which is how far a row may shift to reach a shared column. `max-shift` answers it as a width-spread budget, wherein an alignment rule walks each run in source order and grows a group while its spread stays within the cap, breaking a fresh group at the first row that would exceed it. A positive `N` sets the budget, `0` forbids any shift so every row sits flush, and `false` lifts the cap so a contiguous run folds into one column. [[align-colons]] aligns `:` across its Python contexts, [[align-equals]] does the same for `=` in keyword arguments and assignments, [[align-comparisons]] lines up comparison operators across consecutive lines, [[align-imports]] lines up `as` aliases in `from … import` blocks, and [[align-match-case]] aligns the post-pattern `:` of match arms.

### Toggle-Only Rules

Some rules answer a single yes-or-no question with no parameters worth tuning, so each takes only a bare bool toggle. Reach for `<rule> = false` to silence a rule whose default doesn't fit the project: [[blank-lines]], [[docstring-wrap]], [[legacy-union-syntax]], [[docstring-frame]], [[docstring-expand]], [[step-narration]], [[strip-align-padding]], [[strip-trailing-commas]], and [[unused-future-annotations]].

### Rule-Specific Facets

Other rules read a project-specific input that *Prose* cannot guess from source alone, so each carries the facet shaped for its question. [[alphabetize]] takes `sort-docstring-entries` for the docstring-entry reorder, [[bare-imports]] takes an `allow` list of modules whose bare-import form is preserved alongside an `exempt-aliased` toggle for the alias exemption and a `max-attributes` cap on the distinct-attribute count that draws the lint, [[collection-layout]] takes the `collapse`, `explode`, and `wrap-dict-entries` facets to switch its shape moves on and off independently, plus `max-atomics-per-line` to cap the inline-collection budget and `max-inline-dict-entries` to expand a dict past an entry count, [[call-layout]] takes `max-inline-args` to explode a call past an argument count, [[reassigned-constants]] takes an `allow` list of exempt module-level names, and [[single-use-variables]] takes an `allow-pattern` regex for binding names that opt out of the lint.

## Key Naming

Every key follows one shape so its name predicts its kind. A boolean key reads affirmatively and defaults to `true`, so `key = true` states the behavior that is on. The master switch each rule carries is `enabled`, and a facet gating one pass of its rule takes a verb-led name for the action it governs (*`sort-docstring-entries`, `exempt-aliased`*). No key takes a negative form (*`no-*`, `disable-*`, `skip-*`*) or a polarity-ambiguous bare noun, leaving `false` to always read as *"off."* A parameter key carrying an int, an enum, or a list is a noun for the quantity or set it holds (*`max-shift`, `max-attributes`, `first-party`, `allow`*), because the key names a value rather than gating a behavior.

## Docstring Budgets

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Every Title-case-headed section that follows reads as a code-shaped table and reuses `code-line-length` (*88 by default*) to match surrounding indentation. `docstring-structured-policy` switches them to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The [[docstring-wrap]] rule consumes both budgets.

## Per-Pattern Overrides

A single config carves out per-pattern exceptions through a `[[tool.prose.overrides]]` array-of-tables. Each entry names a `paths` glob list and the partial `[tool.prose]` body its matched files receive, deep-merged per facet over the file's base so the override wins the facets it sets and leaves the rest. A generated directory can relax a budget, or a test suite can drop a lint, without a nested config file at every boundary.

```toml
code-line-length = 88

[[overrides]]
paths            = ["generated/**", "**/_pb2.py"]
code-line-length = 200

[[overrides]]
paths = ["tests/**"]

[overrides.rules]
single-use-variables = false
```

Globs anchor to the declaring config's directory, so `tests/**` matches the `tests/` beside the config rather than at any depth below it. The array-of-tables shape keeps a multi-facet override readable across lines, where a glob-keyed inline table would crowd it onto one against *Prose*'s legibility mandate. When several entries match one file, *Prose* layers their bodies in document order, leaving the last matching entry to win each facet it sets while the earlier entries' other facets stay in place.

## Subset by Invocation

Per-invocation overrides via `--select` and `--ignore` take precedence over the configured-enabled set. See the [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter for the CLI surface, the [**CLI Reference**](/reference/cli) for the full flag list, and the [**Suppression**](/usage/suppression) chapter for per-line opt-outs.
