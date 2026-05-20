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

## Top-Level Keys

The top-level `[tool.prose]` table carries settings that span multiple rules.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `code-line-length` | positive int | `88` | Honored by line-length-aware rules |
| `docstring-line-length` | positive int | `76` | Description-prose budget for [[docstring-wrap]] |
| `docstring-structured-policy` | `"code-line-length"` \| `"docstring-line-length"` | `"code-line-length"` | Source budget for structured docstring sections |
| `target-version` | `"3.X"` version string | unset | Python runtime the project ships to, consumed by version-gated rules |

`target-version` names the Python runtime a project ships to, taking the bare `major.minor` form (*`"3.13"`, `"3.14"`*) used by `mypy`'s `python_version` setting. Rules whose safety depends on the runtime read this field directly, treating an unset value as the cue to skip every version-dependent arm rather than assume a default. [[legacy-union-syntax]] and [[unused-future-annotations]] are the two current consumers, staying quiet on every project that has not opted into a target.

## Per-Rule Knobs

Each rule's sub-table sits at `[tool.prose.rules.<rule>]`. Every rule accepts `enabled` (*defaulting to `true`*), and a handful carry rule-specific knobs.

| Key | Type | Where | Default | Meaning |
|---|---|---|---|---|
| `enabled` | bool | every rule | `true` | Toggle the rule on or off |
| `max-shift` | positive int | alignment rules | `8` | Ceiling on per-line padding |
| `max-shift-policy` | `"split"` \| `"drop"` \| `"skip"` | alignment rules | `"split"` | How to handle a group whose widest member exceeds `max-shift`. `split` partitions the group, `drop` excludes the widest members from the padding calculation, `skip` leaves the whole group unaligned |
| `max-atomics-per-line` | positive int | [[collection-layout]] | `8` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap |
| `allow` | list of module names | [[bare-import-allowlist]] | `["numpy", "pandas"]` | Modules whose bare-import form is preserved |
| `allow` | list of names | [[loose-constants]] | `[]` | Module-level names exempted from the lint |
| `allow-pattern` | regex | [[single-use-variables]] | `"^_"` | Binding names exempted from the lint |

## Rule Categories

The **eighteen** rules sit in three configuration buckets, with each bucket carrying a distinct knob shape.

### Alignment Rules

Carry `max-shift` and `max-shift-policy`, because each one resolves a column-alignment question that may exceed the per-line budget. They are [[align-colons]], [[align-equals]], [[align-imports]], and [[match-case-align]].

### Toggle-Only Rules

Carry only `enabled`. They are [[alphabetize]], [[blank-lines]], [[docstring-wrap]], [[legacy-union-syntax]], [[multi-line-docstrings]], [[no-single-line-docstrings]], [[no-step-narration]], [[singleton-rule]], [[strip-trailing-commas]], and [[unused-future-annotations]].

### Rule-Specific Knobs

Appear on the four rules whose behavior depends on a project-specific list, with allowlists, regexes, or per-line budgets. They are [[bare-import-allowlist]], [[collection-layout]], [[loose-constants]], and [[single-use-variables]].

## Docstring Budgets

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Structured sections (*`Args:`, `Returns:`, `Raises:`*) read as code-shaped tables and reuse `code-line-length` (*88 by default*) to match surrounding indentation. `docstring-structured-policy` switches them to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The [[docstring-wrap]] rule consumes both budgets.

## Subset by Invocation

Per-invocation overrides via `--select` and `--ignore` take precedence over the configured-enabled set. See the [**Quick Start**](/guide/quick-start#subset-the-active-rules) chapter for the CLI surface, the [**CLI Reference**](/reference/cli) for the full flag list, and the [**Suppression**](/guide/suppression) chapter for per-line opt-outs.
