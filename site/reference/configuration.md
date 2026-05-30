# Configuration

*Prose* loads its configuration from a `prose.toml` file or the `[tool.prose]` table of a `pyproject.toml`, walking upward from the working directory to the nearest one. With no configuration, every rule runs at its default, in that a project that writes no config gets the canonical *Prose* shape automatically.

A `prose.toml` keeps its keys at the document root, the form this page shows throughout. A `pyproject.toml` carries the same keys under a `[tool.prose]` prefix so the manifest can house other tools too, leaving every key below a `[tool.prose.<…>]` equivalent for projects that prefer one file.

`target-version` carries the bare `major.minor` form *(`"3.13"`, `"3.14"`)* used by `mypy`'s `python_version` setting, with rules whose safety depends on the runtime reading the field directly. The docstring-budget duality *(`code-line-length` for Title-case-headed structured sections, `docstring-line-length` for description prose)* lets a project keep code-shaped tables wide while keeping description prose at a comfortable reading measure, and `docstring-structured-policy` collapses both to a single budget when a project prefers a uniform width.

To turn rules off or tune them, write the `[rules]` table:

```toml
code-line-length = 88

[rules]
align-colons      = { max-shift = 12, max-shift-policy = "drop" }
align-equals      = false
collection-layout = { max-atomics-per-line = 3 }
```

A bare `false` disables a rule, an inline table sets its knobs while leaving the rule enabled, and a rule you do not name stays on at its default. Under `pyproject.toml` the table reads `[tool.prose.rules]`, and a rule with several knobs may prefer the expanded `[rules.<rule>]` sub-table *(`[tool.prose.rules.<rule>]` in the manifest)*, which carries the same settings as the inline form.

## Where Prose Looks

*Prose* walks upward from the working directory toward the filesystem root. In each directory a `prose.toml` outranks a `pyproject.toml`, and the nearest directory carrying either wins, in that *Prose* reads only that one file and never merges across matches up the tree. A `pyproject.toml` lacking a `[tool.prose]` table is passed over, leaving the walk to continue upward. When no ancestor carries either, every default applies as if the config were empty.

When a `prose.toml` and a `pyproject.toml` `[tool.prose]` table share a directory, the `prose.toml` wins and *Prose* notes the precedence to stderr, so the file that took effect is never ambiguous.

## Top-Level Keys

The top-level keys carry settings that span multiple rules. They sit at the document root in a `prose.toml` and under `[tool.prose]` in a `pyproject.toml`.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `code-line-length` | positive int | `88` | Honored by line-length-aware rules |
| `docstring-line-length` | positive int | `76` | Description-prose budget for [[docstring-wrap]] |
| `docstring-structured-policy` | `"code-line-length"` \| `"docstring-line-length"` | `"code-line-length"` | Source budget for structured docstring sections |
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

## Per-Rule Knobs

The `[rules]` table holds one entry per rule you change. A bare bool is the shorthand for `enabled` (*`alphabetize = false`*), an inline table sets a rule's knobs (*`align-equals = { max-shift = 4 }`*), and a rule you do not name stays enabled at its defaults. The table below lists the knobs each rule accepts, with the *Where* column resolving which rules share a knob. Generic knobs *(`enabled`, `max-shift`, `max-shift-policy`)* apply to every rule in their column's named category. Rule-specific knobs read inputs scoped to the named rule, even when two rules happen to spell their knob the same way *(the two `allow` rows are distinct, scoped to different rules and reading different inputs)*.

| Key | Type | Where | Default | Meaning |
|---|---|---|---|---|
| `enabled` | bool | every rule | `true` | Toggle the rule on or off |
| `max-shift` | positive int | alignment rules | `8` | Ceiling on per-line padding |
| `max-shift-policy` | `"split"` \| `"drop"` | alignment rules | `"split"` | How to handle a group whose widest member exceeds `max-shift`. `split` partitions the group, `drop` excludes the widest members from the padding calculation. To leave one row out of an otherwise-aligned group, hold it with `# prose: skip` rather than abandoning the alignment |
| `docstring-entries` | bool | [[alphabetize]] | `true` | Reorder `name: description` entries within every Title-case-headed docstring section alongside the AST-level sorts. Set `false` to keep narrative-curated entry order while still sorting every other surface |
| `max-atomics-per-line` | positive int | [[collection-layout]] | `8` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap |
| `allow` | list of module names | [[bare-import-allowlist]] | `["numpy", "pandas"]` | Modules whose bare-import form is preserved |
| `allow` | list of names | [[loose-constants]] | `[]` | Module-level names exempted from the lint |
| `allow-pattern` | regex | [[single-use-variables]] | `"^_"` | Binding names exempted from the lint |

::: warning `allow` Replaces the Default
A user-supplied `allow` list replaces the rule's default rather than extending it. A project that wants its own modules alongside `bare-import-allowlist`'s bundled `"numpy"` and `"pandas"` must list those two explicitly in the supplied `allow` array, otherwise the default falls away.
:::

## Rule Categories

Rules sit in configuration buckets, with each bucket carrying a distinct knob shape.

### Alignment Rules

The four rules that line columns vertically share one structural question: what happens when the widest member would push the alignment column past `code-line-length`. `max-shift` caps the leftward shift and `max-shift-policy` names the fallback the rule reaches for when the cap binds. [[align-colons]] aligns `:` in five Python contexts, [[align-equals]] does the same for `=` in keyword arguments and assignments, [[align-imports]] lines up `as` aliases in `from … import` blocks, and [[match-case-align]] aligns the `->` arrows of match arms.

### Toggle-Only Rules

Some rules answer a single yes-or-no question with no parameters worth tuning, so each takes only a bare bool toggle. Reach for `<rule> = false` to silence a rule whose default doesn't fit the project: [[blank-lines]], [[docstring-wrap]], [[legacy-union-syntax]], [[multi-line-docstrings]], [[no-single-line-docstrings]], [[no-step-narration]], [[singleton-rule]], [[strip-trailing-commas]], and [[unused-future-annotations]].

### Rule-Specific Knobs

Other rules read a project-specific input that *Prose* cannot guess from source alone, so each carries the knob shaped for its question. [[alphabetize]] takes `docstring-entries` for the docstring-entry reorder, [[bare-import-allowlist]] takes an `allow` list of modules whose bare-import form is preserved, [[collection-layout]] takes `max-atomics-per-line` to cap the inline-collection budget, [[loose-constants]] takes an `allow` list of exempt module-level names, and [[single-use-variables]] takes an `allow-pattern` regex for binding names that opt out of the lint.

## Docstring Budgets

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Every Title-case-headed section that follows reads as a code-shaped table and reuses `code-line-length` (*88 by default*) to match surrounding indentation. `docstring-structured-policy` switches them to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The [[docstring-wrap]] rule consumes both budgets.

## Subset by Invocation

Per-invocation overrides via `--select` and `--ignore` take precedence over the configured-enabled set. See the [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter for the CLI surface, the [**CLI Reference**](/reference/cli) for the full flag list, and the [**Suppression**](/usage/suppression) chapter for per-line opt-outs.
