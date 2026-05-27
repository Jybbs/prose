# Configuration

*Prose* loads the nearest `[tool.prose]` table found by walking upward from the working directory. The table is the canonical look-up surface for every project-wide knob *Prose* exposes. With no configuration, every rule runs at its default, in that a project that doesn't write a `[tool.prose]` table gets the canonical *Prose* shape automatically.

`target-version` carries the bare `major.minor` form *(`"3.13"`, `"3.14"`)* used by `mypy`'s `python_version` setting, with rules whose safety depends on the runtime reading the field directly. The docstring-budget duality *(`code-line-length` for structured `Args:` / `Returns:` / `Raises:` sections, `docstring-line-length` for description prose)* lets a project keep code-shaped tables wide while keeping description prose at a comfortable reading measure, and `docstring-structured-policy` collapses both to a single budget when a project prefers a uniform width.

To tune a rule, write its sub-table inside `pyproject.toml`:

```toml
[tool.prose]
code-line-length = 88

[tool.prose.rules.align-equals]
enabled = false

[tool.prose.rules.align-colons]
max-shift        = 12
max-shift-policy = "drop"

[tool.prose.rules.collection-layout]
max-atomics-per-line = 3
```

## Where Prose Looks

*Prose* walks upward from the working directory looking for a `pyproject.toml` file. The first one it finds wins, in that *Prose* reads only that file's `[tool.prose]` table and never merges across multiple matches up the tree. When no ancestor carries a `pyproject.toml`, every default applies as if the table were empty. No alternate filenames are read *(no `prose.toml`, no `.prose.toml`)*, because the `[tool.prose]` table in `pyproject.toml` already gives the project a single canonical home for the configuration.

## Top-Level Keys

The top-level `[tool.prose]` table carries settings that span multiple rules.

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

## Per-Rule Knobs

Each rule's sub-table sits at `[tool.prose.rules.<rule>]`. Every rule accepts `enabled` (*defaulting to `true`*), and a handful carry rule-specific knobs. The table below mixes two kinds of row, with the *Where* column resolving the difference. Generic knobs *(`enabled`, `max-shift`, `max-shift-policy`)* apply to every rule in their column's named category. Rule-specific knobs sit at the named rule's sub-table only, even when two rules happen to spell their knob the same way *(the two `allow` rows are distinct, scoped to different rules and reading different inputs)*.

| Key | Type | Where | Default | Meaning |
|---|---|---|---|---|
| `enabled` | bool | every rule | `true` | Toggle the rule on or off |
| `max-shift` | positive int | alignment rules | `8` | Ceiling on per-line padding |
| `max-shift-policy` | `"split"` \| `"drop"` \| `"skip"` | alignment rules | `"split"` | How to handle a group whose widest member exceeds `max-shift`. `split` partitions the group, `drop` excludes the widest members from the padding calculation, `skip` leaves the whole group unaligned |
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

Some rules answer a single yes-or-no question with no parameters worth tuning, so each ships with `enabled` and nothing else. Reach for the toggle to silence a rule whose default doesn't fit the project: [[blank-lines]], [[docstring-wrap]], [[legacy-union-syntax]], [[multi-line-docstrings]], [[no-single-line-docstrings]], [[no-step-narration]], [[singleton-rule]], [[strip-trailing-commas]], and [[unused-future-annotations]].

### Rule-Specific Knobs

Other rules read a project-specific input that *Prose* cannot guess from source alone, so each carries the knob shaped for its question. [[alphabetize]] takes `docstring-entries` for the docstring-entry reorder, [[bare-import-allowlist]] takes an `allow` list of modules whose bare-import form is preserved, [[collection-layout]] takes `max-atomics-per-line` to cap the inline-collection budget, [[loose-constants]] takes an `allow` list of exempt module-level names, and [[single-use-variables]] takes an `allow-pattern` regex for binding names that opt out of the lint.

## Docstring Budgets

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Structured sections (*`Args:`, `Returns:`, `Raises:`*) read as code-shaped tables and reuse `code-line-length` (*88 by default*) to match surrounding indentation. `docstring-structured-policy` switches them to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The [[docstring-wrap]] rule consumes both budgets.

## Subset by Invocation

Per-invocation overrides via `--select` and `--ignore` take precedence over the configured-enabled set. See the [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter for the CLI surface, the [**CLI Reference**](/reference/cli) for the full flag list, and the [**Suppression**](/usage/suppression) chapter for per-line opt-outs.
