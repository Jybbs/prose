# Configuration

*Prose* loads its configuration from a `prose.toml` file, a `.config/prose.toml`, or the `[tool.prose]` table of a `pyproject.toml`, walking upward from each input file's directory to the nearest one. With no configuration, every rule runs at its default, in that a project that writes no config gets the canonical *Prose* shape automatically. The whole key set is also available as a machine-readable [JSON Schema](https://json-schema.org) through [`prose schema`](/reference/cli#prose-schema).

A `prose.toml` keeps its keys at the document root, the form this page shows throughout, and a `.config/prose.toml` reads the same way for a project that keeps its tool config under a `.config/` directory. A `pyproject.toml` carries the same keys under a `[tool.prose]` prefix so the manifest can house other tools too, leaving every key below a `[tool.prose.<…>]` equivalent for projects that prefer one file.

`target-version` carries the bare `major.minor` form *(`"3.13"`, `"3.14"`)* used by `mypy`'s `python_version` setting, with rules whose safety depends on the runtime reading the field directly. The docstring-budget duality *(`code-line-length` for Title-case-headed structured sections, `docstring-line-length` for description prose)* lets a project keep code-shaped tables wide while keeping description prose at a comfortable reading measure, and `docstring-structured-policy` collapses both to a single budget when a project prefers a uniform width.

To turn rules off or tune them, write the `[rules]` table:

```toml
code-line-length = 88

[rules]
align-colons      = { max-shift = false }
align-equals      = false
collection-layout = { max-atomics = 3 }
```

A bare `false` disables a rule, an inline table sets its facets while leaving the rule enabled, and a rule you do not name stays on at its default. Under `pyproject.toml` the table reads `[tool.prose.rules]`, and a rule with several facets may prefer the expanded `[rules.<rule>]` sub-table *(`[tool.prose.rules.<rule>]` in the manifest)*, which carries the same settings as the inline form.

## Where *Prose* Looks

*Prose* walks upward from each input file's own directory toward the filesystem root, so a file answers to its own project's config even when one invocation names files across several projects. Stdin input walks from the working directory, the one input with no path of its own. In each directory a `prose.toml` outranks a `.config/prose.toml`, which outranks a `pyproject.toml`, and the nearest directory carrying any of them wins, in that *Prose* reads only that one file and never merges across matches up the tree. A `pyproject.toml` lacking a `[tool.prose]` table is passed over, leaving the walk to continue upward. A standalone script the walk never resolves to a project reads its own `[tool.prose]` from a leading PEP 723 `# /// script` block, the one configuration home a single-file script has, whereas a script under a project ignores its block and answers to the project. When neither an ancestor nor a block carries config, every default applies as if the config were empty.

When more than one of these forms share a directory, the higher-precedence one wins and *Prose* notes the precedence to stderr, so the file that took effect is never ambiguous.

## Top-Level Keys

The top-level keys carry settings that span multiple rules. They sit at the document root in a `prose.toml` and under `[tool.prose]` in a `pyproject.toml`.

<ConfigKeys section="top" />

`target-version` names the Python runtime a project ships to, taking the bare `major.minor` form (*`"3.13"`, `"3.14"`*) used by `mypy`'s `python_version` setting. Rules whose safety depends on the runtime read this field directly. [[legacy-union-syntax]] and [[unused-future-annotations]] are the two current consumers.

::: info Version Gates Need Opt-In
With no value set, every version-dependent arm skips rather than assume a default, leaving [[legacy-union-syntax]] and [[unused-future-annotations]] quiet on every project that has not opted into a target.
:::

## Lengths

The `*-line-length` caps are hard constraints, and every shaping rule resolves within them rather than reading the budget as a hint. `code-line-length` governs code lines and `import-line-length` governs import lines, with the count knobs *(`max-args`, `max-params`, `max-dict-entries`)* choosing shapes only for the lines that already fit beneath a cap.

A construct with a legal multi-line reshape takes it once its line crosses the cap, whatever a count threshold says, so a call over `code-line-length` explodes to one argument per line even at or under `max-args`, and a signature, collection, or `from` import does the same against its budget. An alignment run whose padding would carry a member past its cap reshapes that member first *(an import splits per [[import-layout]], a call or collection value explodes per its layout rule)* and then aligns within the cap, a member partitioning out of the run unpadded the way an over-`max-shift` outlier does only when no reshape can bring its aligned width under.

A cap no legal form can satisfy *(a deep indent, a long identifier, a cap set below what a statement needs)* leaves the narrowest legal form standing, and [[line-overflow]] names that remainder, so an unsatisfiable cap reads as a finding in `prose check` and a squiggle in the sandbox rather than as a knob that did nothing.

## Cache

The `[cache]` table tunes the user-level [**cache**](/reference/cache) that *Prose* keeps for repeat runs *(`[tool.prose.cache]` in a `pyproject.toml`)*. Both keys default to the canonical shape, so a project that does not write the table gets the cache at its full size.

<ConfigKeys section="cache" />

```toml
[cache]
enabled      = true
max-size-mib = 250
```

## Imports

The `[imports]` table names the project's first-party packages *(`[tool.prose.imports]` in a `pyproject.toml`)*, so [[group-imports]] places their imports with relative imports in the local-package section rather than the external `from` section. With no list, only relative imports (`from .`, `from ..pkg`) populate the local-package section.

<ConfigKeys section="imports" />

```toml
[imports]
first-party = ["myapp", "acme"]
```

A list entry names a root package, so `myapp` matches `import myapp.db` and `from myapp import app` while leaving `from myapplication import x` in the external `from` group.

## Per-Rule Facets

The `[rules]` table holds one entry per rule you change. A bare bool is the shorthand for `enabled` (*`alphabetize = false`*), an inline table sets a rule's facets (*`align-equals = { max-shift = 4 }`*), and a rule you do not name stays enabled at its defaults. The facets below group by rule family and nest under the rule that reads each one, so the two `allow` facets stay distinct because they belong to different rules and read different inputs. The Generic group gathers the facets that cut across families, wherein `enabled` reaches every rule and `max-shift` every alignment rule.

<PerRuleFacets />

## Rule Categories

Every rule is either auto-fix or lint, and that split decides what a diagnostic does rather than how a rule is configured. An auto-fix rule rewrites the source under `prose format` and reports the pending edit under `prose check`, whereas a lint rule only reports and leaves the change to a human, because its fix turns on a judgment *Prose* declines to make. The distinction sets the `prose check` exit code, wherein a pending auto-fix returns `1` and a lint violation returns `2`, so a CI gate tells the two apart. Configuration stays identical across the split, leaving `<rule> = false` to silence either kind and the facets above to tune either kind.

## Key Naming

Every key follows one shape so its name predicts its kind. A boolean key reads affirmatively and defaults to `true`, so `key = true` states the behavior that is on. The master switch each rule carries is `enabled`, and a facet gating one pass of its rule takes a verb-led name for the action it governs (*`sort-docstring-entries`, `exempt-aliased`*). No key takes a negative form (*`no-*`, `disable-*`, `skip-*`*) or a polarity-ambiguous bare noun, leaving `false` to always read as *"off."* A parameter key carrying an int, an enum, or a list is a noun for the quantity or set it holds (*`max-shift`, `max-attributes`, `first-party`, `allow`*), because the key names a value rather than gating a behavior.

## Docstring Budgets

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Every Title-case-headed section that follows reads as a code-shaped table whose prose lines reuse `code-line-length` (*88 by default*) to match surrounding indentation, whereas its `name: description` entries wrap to `docstring-line-length` with a hanging indent at the description's start column. `docstring-structured-policy` switches the prose lines to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The [[docstring-wrap]] rule consumes both budgets.

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
