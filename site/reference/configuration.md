---
description: "Covers the `prose.toml`, `.config/prose.toml`, and `pyproject.toml` config files and every per-rule facet."
---

# Configuration

*Prose* reads its configuration from a `prose.toml` file, a `.config/prose.toml`, or the `[tool.prose]` table of a `pyproject.toml`, as [**The Config Search Order**](#the-config-search-order) sets out. With no configuration, every rule runs at its default, so a project that writes no config gets the standard *Prose* layout. The whole key set is also available as a [JSON Schema](https://json-schema.org) through [`prose schema`](/reference/cli#prose-schema).

A `prose.toml` keeps its keys at the document root, which is the form this page shows throughout, and a `.config/prose.toml` reads the same way for a project that keeps its tool config under `.config/`. A `pyproject.toml` carries the same keys under a `[tool.prose]` prefix, so every key below has a `[tool.prose.<…>]` equivalent for a project that keeps one manifest.

To turn rules off or adjust them, write the `[rules]` table:

```toml
code-line-length = 88

[rules]
align-colons       = { max-shift = false }
align-equals       = false
reflow-collections = { max-atomics = 3 }
```

A bare `false` turns a rule off as the shorthand for `enabled = false`, an inline table sets its facets and leaves the rule on, and a rule you do not name stays on at its default. Under `pyproject.toml` the table reads `[tool.prose.rules]`, and a rule with several facets may read better as a `[rules.<rule>]` sub-table (*`[tool.prose.rules.<rule>]` in the manifest*), which takes the same settings as the inline form.

## The Config Search Order

*Prose* searches upward from each input file's own directory toward the filesystem root, so a file follows its own project's config even when one run names files from several projects, and stdin input searches from the directory of the file `--stdin-filename` names, or from the working directory when it names none. The search settles on the nearest directory holding any of these, ranked in this order within a directory:

1. `prose.toml`
2. `.config/prose.toml`
3. `pyproject.toml`, skipped when it carries no `[tool.prose]` table

*Prose* reads that one file and never merges files from further up the tree. A standalone script that belongs to no project reads its own `[tool.prose]` from a leading PEP 723 `# /// script` block, the one place a single-file script can hold config, whereas a script inside a project ignores its block and follows the project. When neither an ancestor directory nor a block carries config, every default applies.

When more than one of these files share a directory, the higher-ranked one wins and *Prose* prints a note to stderr naming it, so the file in effect is never ambiguous.

## Top-Level Keys

The top-level keys hold settings that reach several rules.

<ConfigKeys section="top" />

`target-version` names the Python version the project runs on, in the bare `major.minor` form (*`"3.13"`, `"3.14"`*) that `mypy`'s `python_version` setting uses. Every rule whose rewrite depends on the runtime reads it, which covers [[band-constants]], [[modernize-annotations]], [[prefer-fstring]], and [[prune-inert-imports]]. A file holding syntax the version does not support, such as a `lazy` import under `"3.14"`, or a semantic syntax error such as a `lazy` import inside a function, fails at exit code `3` naming the file, the position, and the version, unless formatting rewrites the error away. A file a `# prose: off` or `# fmt: off` at its head suppresses skips that check, and a `target-version` under [**Per-Pattern Overrides**](#per-pattern-overrides) holds the files a glob matches to a version of their own.

::: info Version Gates Need Opt-In
With no value set, every version-dependent rewrite is skipped rather than assuming a default, so the version-gated rules do nothing on a project that has not set a target, a name counts as a builtin only where Python 3.10 already defines it, and only syntax every Python version rejects fails a file.
:::

The [**Unstable Output**](/reference/cli#unstable-output) section of the CLI reference covers the notice `report-unstable-output` governs.

## Lengths

The `*-line-length` caps are hard limits, and every rule that lays out code fits within them rather than reading the budget as a hint. `code-line-length` governs code lines, `import-line-length` governs import lines, and `docstring-line-length` governs docstring description prose and section entries, whereas the count keys (*`max-args`, `max-params`, `max-dict-entries`, `max-links`*) choose a layout only for lines that already fit under a cap.

A construct with a legal multi-line layout takes it once its line crosses the cap, whatever its count says, so a call over `code-line-length` explodes to one argument per line even at or under `max-args`, a signature expands to one parameter per line and a collection across rows, and a `from` import splits into a run of statements against `import-line-length`. Where padding would carry a row past its cap, a layout rule lays out the row's import or value first so the row aligns within the cap. [[reflow-imports]] splits an import in an [[align-imports]] run, and the [[align-colons]] and [[align-equals]] pages set out which value a layout rule breaks open in a dict entry or an assignment. Where no layout rule brings a row under the cap, the run splits before the first row whose joining would carry a row past it, and the rows from there align as a column of their own, the same cut `max-shift` makes. A `case` arm is the exception, since [[align-match-case]] leaves an arm unfolded with its `:` flush against the pattern where its folded line would cross the cap.

Several alignment rules reach the same row, since [[align-colons]], [[align-equals]], and [[align-comments]] each place a column on a line carrying an annotation, a value, and a comment. Each rule measures the cap against the line it will write rather than the line as it stands, counting a trailing comment at its two-space gap and the space after an operator at the single space an aligned row carries, both of which a later rule sets. Each rule's fit decision is therefore unchanged by the rules that run after it, so the columns resolve in one pass rather than shifting across repeated runs.

A trailing comment counts toward the cap like any other span on the line, since *Prose* places it through [[normalize-comment-spacing]] and [[align-comments]] rather than leaving it untouched. A row already past its cap before any padding therefore keeps its own gap and joins a shared column only where that column adds no width to it, which only the widest row of a run satisfies. Alignment never pushes an over-budget line further out, and [[line-overflow]] reports the remainder at the narrowest width the row can reach. A layout rule measures a row only up to its trailing comment, though, so a line whose code fits while its comment runs past the cap keeps its layout, and the [[line-overflow]] page sets out when that line is reported.

A cap no legal layout can meet (*a deep indent, a long identifier, a cap set below what a statement needs*) leaves the narrowest legal layout in place, and [[line-overflow]] reports what remains.

## Cache

The `[cache]` table (*`[tool.prose.cache]` in a `pyproject.toml`*) tunes the per-user cache *Prose* keeps for repeat runs, and the [**Cache**](/reference/cache#configuration) reference lists its keys and their defaults.

## Imports

The `[imports]` table names the project's first-party packages (*`[tool.prose.imports]` in a `pyproject.toml`*), so [[group-imports]] places their imports with the relative imports in the local-package section rather than the external `from` section. With no list, only relative imports (`from .`, `from ..pkg`) fill the local-package section.

<ConfigKeys section="imports" />

```toml
[imports]
first-party = ["myapp", "acme"]
```

A list entry names a root package, so `myapp` matches `import myapp.db` and `from myapp import app` and leaves `from myapplication import x` in the external `from` group.

## Per-Rule Facets

The `[rules]` table holds one entry per rule you change, in the forms the top of this page shows. The facets below are grouped by rule family and nested under the rule that reads each one, so the two `allow` facets stay distinct because they belong to different rules and take different inputs. The Generic group gathers the facets several rules share, since `enabled` reaches every rule and `max-shift` every alignment rule but [[strip-stranded-padding]].

<PerRuleFacets />

## Key Naming

Every key follows one shape, so its name predicts its kind:

- A boolean key reads affirmatively and defaults to `true`, so `key = true` names the behavior that is on.
- The master switch every rule carries is `enabled`, and a facet governing one pass of its rule takes a verb-led name for the action it governs (*`sort-docstring-entries`, `exempt-aliased`*).
- No key takes a negative form (*`no-*`, `disable-*`, `skip-*`*) or a bare noun whose polarity is unclear, so `false` always reads as *"off."*
- A parameter key holding an int, an enum, or a list is a noun for the quantity or set it holds (*`max-shift`, `max-attributes`, `first-party`, `allow`*), because the key names a value rather than switching a behavior.

## Docstring Budgets

Two budgets govern the text [[wrap-docstrings]] wraps inside a docstring. `docstring-line-length` (*<ConfigDefault facet="docstring-line-length" /> by default*) holds the description prose between the opening `"""` and the first section heading, along with every section's `name: description` entries. The prose lines of each Title-case-headed section take `code-line-length` (*<ConfigDefault facet="code-line-length" /> by default*), and `docstring-structured-policy` switches them to `docstring-line-length` for a project that prefers one narrower budget across the whole docstring.

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

`--select` and `--ignore` on the command line take precedence over the configured rule set for one run, as the [**Precedence**](/reference/cli#precedence) section of the CLI reference covers.
