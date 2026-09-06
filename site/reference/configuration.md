---
description: "Covers the `prose.toml`, `.config/prose.toml`, and `pyproject.toml` config files and every per-rule facet."
---

# Configuration

*Prose* reads its configuration from a `prose.toml` file, a `.config/prose.toml`, or the `[tool.prose]` table of a `pyproject.toml`, searching upward from each input file's directory to the nearest one. With no configuration, every rule runs at its default, so a project that writes no config gets the standard *Prose* layout. The whole key set is also available as a [JSON Schema](https://json-schema.org) through [`prose schema`](/reference/cli#prose-schema).

A `prose.toml` keeps its keys at the document root, which is the form this page shows throughout, and a `.config/prose.toml` reads the same way for a project that keeps its tool config under `.config/`. A `pyproject.toml` carries the same keys under a `[tool.prose]` prefix, so every key below has a `[tool.prose.<…>]` equivalent for a project that keeps one manifest.

The top-level keys hold the settings that reach several rules. `target-version` takes the bare `major.minor` form *(`"3.13"`, `"3.14"`)* that `mypy`'s `python_version` setting uses, and the rules whose rewrites depend on the runtime read it directly. The two docstring budgets *(`code-line-length` for Title-case-headed structured sections, `docstring-line-length` for description prose)* let a project keep code-shaped tables wide and description prose at a comfortable reading width, and `docstring-structured-policy` puts both under one budget for a project that prefers one width.

To turn rules off or adjust them, write the `[rules]` table:

```toml
code-line-length = 88

[rules]
align-colons      = { max-shift = false }
align-equals      = false
reflow-collections = { max-atomics = 3 }
```

A bare `false` turns a rule off, an inline table sets its facets and leaves the rule on, and a rule you do not name stays on at its default. Under `pyproject.toml` the table reads `[tool.prose.rules]`, and a rule with several facets may read better as a `[rules.<rule>]` sub-table *(`[tool.prose.rules.<rule>]` in the manifest)*, which takes the same settings as the inline form.

## Where *Prose* Looks

*Prose* searches upward from each input file's own directory toward the filesystem root, so a file follows its own project's config even when one run names files from several projects, and stdin input searches from the working directory, since it has no path of its own. The search settles on the nearest directory holding any of these, ranked in this order within a directory:

1. `prose.toml`
2. `.config/prose.toml`
3. `pyproject.toml`, skipped when it carries no `[tool.prose]` table

*Prose* reads that one file and never merges files from further up the tree. A standalone script that belongs to no project reads its own `[tool.prose]` from a leading PEP 723 `# /// script` block, the one place a single-file script can hold config, whereas a script inside a project ignores its block and follows the project. When neither an ancestor directory nor a block carries config, every default applies.

When more than one of these files share a directory, the higher-ranked one wins and *Prose* prints a note to stderr naming it, so the file in effect is never ambiguous.

## Top-Level Keys

The top-level keys hold settings that reach several rules. They sit at the document root in a `prose.toml` and under `[tool.prose]` in a `pyproject.toml`.

<ConfigKeys section="top" />

`target-version` names the Python version the project runs on, in the bare `major.minor` form *(`"3.13"`, `"3.14"`)* that `mypy`'s `python_version` setting uses. Every rule whose rewrite depends on the runtime reads it, which covers [[modernize-annotations]], [[prefer-fstring]], and [[prune-inert-imports]].

::: info Version Gates Need Opt-In
With no value set, every version-dependent rewrite is skipped rather than assuming a default, so the version-gated rules do nothing on a project that has not set a target.
:::

`report-unstable-output` governs what a `format` run does when a file it just rewrote is one a second run would change again. When on, the run prints an [**unstable-output notice**](/reference/cli#unstable-output) naming the rules that disagree and the command that reproduces the defect, while still writing the rewrite and still setting the exit code from that rewrite alone, since a layout defect belongs to *Prose* rather than to the source. When off, the rewrite is written with no notice, whereas [`prose check --validate`](/reference/cli#prose-check) keeps its settle check, because the key governs the notice rather than the check the flag turns on. The notice also reaches an editor through [`prose server`](/reference/cli#prose-server), once per document per session rather than on every save.

The two checks differ in reach, in that a `format` run, and the editor message [`prose server`](/reference/cli#prose-server) sends, re-apply only the rules that edited on the first pass, whereas `prose check --validate` re-applies every enabled rule. A rule that was silent on the first pass and would still change the output therefore goes unreported by the notice and is caught instead by `prose check --validate`, by a later cached run that finds its own earlier output rewritten, and by the corpus sweeps that run over the standard library and its mutations.

## Lengths

The `*-line-length` caps are hard limits, and every rule that lays out code fits within them rather than reading the budget as a hint. `code-line-length` governs code lines and `import-line-length` governs import lines, and the count keys *(`max-args`, `max-params`, `max-dict-entries`, `max-links`)* choose a layout only for lines that already fit under a cap.

A construct with a legal multi-line layout takes it once its line crosses the cap, whatever its count says, so a call over `code-line-length` explodes to one argument per line even at or under `max-args`, and a signature, collection, or `from` import does the same against its own budget. An alignment run whose padding would push a row past its cap lays that row out first *(an import splits per [[reflow-imports]], a call or collection value explodes per its layout rule)* and then aligns within the cap. A row leaves the run unpadded, the way a row over `max-shift` does, only when no layout can bring its aligned width under the cap.

Several alignment rules reach the same row, since [[align-colons]], [[align-equals]], and [[align-comments]] each place a column on a line carrying an annotation, a value, and a comment. Each rule measures the cap against the line it will write rather than the line as it stands, counting a trailing comment at its two-space gap and the space after an operator at the single space an aligned row carries, both of which a later rule sets. Each rule's fit decision is therefore unchanged by the rules that run after it, so the columns resolve in one pass rather than shifting across repeated runs.

A trailing comment counts toward the cap like any other span on the line, since *Prose* places it through [[normalize-comment-spacing]] and [[align-comments]] rather than leaving it untouched. A row already past its cap before any padding therefore keeps its own gap and joins a shared column only where that column adds no width to it, which only the widest row of a run satisfies. Alignment never pushes an over-budget line further out, and [[line-overflow]] reports the remainder at the narrowest width the row can reach.

A cap no legal layout can meet *(a deep indent, a long identifier, a cap set below what a statement needs)* leaves the narrowest legal layout in place, and [[line-overflow]] reports what remains, so an unsatisfiable cap shows up as a finding in `prose check` and a flagged line in the sandbox rather than as a setting that did nothing.

## Cache

The `[cache]` table tunes the per-user [**cache**](/reference/cache) *Prose* keeps for repeat runs *(`[tool.prose.cache]` in a `pyproject.toml`)*. Every key defaults to its standard value, so a project that does not write the table gets the cache at its full size.

<ConfigKeys section="cache" />

```toml
[cache]
enabled      = true
max-entries  = 25000
max-size-mib = 250
```

## Imports

The `[imports]` table names the project's first-party packages *(`[tool.prose.imports]` in a `pyproject.toml`)*, so [[group-imports]] places their imports with the relative imports in the local-package section rather than the external `from` section. With no list, only relative imports (`from .`, `from ..pkg`) fill the local-package section.

<ConfigKeys section="imports" />

```toml
[imports]
first-party = ["myapp", "acme"]
```

A list entry names a root package, so `myapp` matches `import myapp.db` and `from myapp import app` and leaves `from myapplication import x` in the external `from` group.

## Per-Rule Facets

The `[rules]` table holds one entry per rule you change. A bare bool is the shorthand for `enabled` (*`alphabetize-siblings = false`*), an inline table sets a rule's facets (*`align-equals = { max-shift = 4 }`*), and a rule you do not name stays on at its defaults. The facets below are grouped by rule family and nested under the rule that reads each one, so the two `allow` facets stay distinct because they belong to different rules and take different inputs. The Generic group gathers the facets several rules share, since `enabled` reaches every rule and `max-shift` every alignment rule.

<PerRuleFacets />

## Rule Categories

Every rule is either auto-fix or lint, and that split decides what a diagnostic does rather than how a rule is configured. An auto-fix rule rewrites the source under `prose format` and reports the pending edit under `prose check`, whereas a lint rule only reports and leaves the change to a person, because its fix depends on a judgment *Prose* does not make. The split sets the `prose check` exit code, `1` for a pending auto-fix and `2` for a lint finding, so a CI gate can tell the two apart. Configuration is the same on both sides, so `<rule> = false` turns either kind off and the facets above tune either kind.

## Key Naming

Every key follows one shape, so its name predicts its kind:

- A boolean key reads affirmatively and defaults to `true`, so `key = true` names the behavior that is on.
- The master switch every rule carries is `enabled`, and a facet governing one pass of its rule takes a verb-led name for the action it governs (*`sort-docstring-entries`, `exempt-aliased`*).
- No key takes a negative form (*`no-*`, `disable-*`, `skip-*`*) or a bare noun whose polarity is unclear, so `false` always reads as *"off."*
- A parameter key holding an int, an enum, or a list is a noun for the quantity or set it holds (*`max-shift`, `max-attributes`, `first-party`, `allow`*), because the key names a value rather than switching a behavior.

## Docstring Budgets

A docstring holds two kinds of text inside one triple-quoted region. The description prose between the opening `"""` and the first section heading reads as paragraphs and wraps to `docstring-line-length` (*<ConfigDefault facet="docstring-line-length" /> by default*), a comfortable line for sustained reading. Each Title-case-headed section after it reads as a code-shaped table, whose prose lines take `code-line-length` (*<ConfigDefault facet="code-line-length" /> by default*) to match the surrounding code, whereas its `name: description` entries wrap to `docstring-line-length` with a hanging indent at the column the description starts on. `docstring-structured-policy` switches those prose lines to `docstring-line-length` for a project that prefers one narrower budget across the whole docstring. [[wrap-docstrings]] reads both budgets.

## Per-Pattern Overrides

A single config can carve out exceptions by path through a `[[tool.prose.overrides]]` array of tables. Each entry names a `paths` glob list and the partial `[tool.prose]` body its matching files receive, merged facet by facet over the file's base config, so the override sets the facets it names and leaves the rest. A generated directory can widen a budget, or a test suite can drop a lint, without a nested config file at every boundary.

```toml
code-line-length = 88

[[overrides]]
paths            = ["generated/**", "**/_pb2.py"]
code-line-length = 200

[[overrides]]
paths = ["tests/**"]

[overrides.rules]
inlinable-bindings = false
```

Globs are relative to the directory of the config that declares them, so `tests/**` matches the `tests/` beside the config and not a `tests/` at any depth below it. The array-of-tables form keeps an override with several facets readable across lines, where an inline table keyed by glob would crowd them onto one. When several entries match one file, *Prose* applies their bodies in document order, so the last matching entry wins each facet it sets and the earlier entries' other facets stay in effect.

## Subset by Invocation

`--select` and `--ignore` on the command line take precedence over the configured rule set for one run. The [**Quick Start**](/usage/quick-start#subset-the-active-rules) chapter covers the flags, the [**CLI Reference**](/reference/cli) the full flag list, and the [**Suppression**](/usage/suppression) chapter the per-line opt-outs.
