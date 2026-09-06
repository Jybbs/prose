import type { InlineNode } from '../markdown/inline-nodes'
import { DIRECTIVES }      from '../suppression/directives'
import { directiveHref }   from '../suppression/scopes'

export type Domain =
  | 'cli-flag'
  | 'config-key'
  | 'exit-code'
  | 'output-format'
  | 'subcommand'
  | 'suppression'

interface TokenSource {
  blurb : string
  href  : string
  key   : string
}

export interface Token {
  blurbNodes : InlineNode[]
  domain     : Domain
  href       : string
  key        : string
  sort       : string
}

export const DOMAIN_LABELS: Record<Domain, string> = {
  'cli-flag'      : 'CLI Flag',
  'config-key'    : 'Configuration Key',
  'exit-code'     : 'Exit Code',
  'output-format' : 'Output Format',
  'subcommand'    : 'Subcommand',
  'suppression'   : 'Suppression Directive'
}

export const SOURCES: Record<Domain, readonly TokenSource[]> = {
  'cli-flag': [
    { key: '--color',          href: '/reference/cli#global-flags',         blurb: 'Whether to color human-readable output.' },
    { key: '--diff',           href: '/reference/cli#prose-format',         blurb: 'Print a unified diff instead of rewriting the source.' },
    { key: '--ignore <slug>',  href: '/reference/cli#precedence',           blurb: 'Remove the listed rule from the active set.' },
    { key: '--no-cache',       href: '/reference/cache',                    blurb: 'Bypass the per-user cache for one run.' },
    { key: '--output-format',  href: '/reference/cli#prose-format',         blurb: 'Choose the diagnostic format (`text` / `json` / `github` / `sarif`).' },
    { key: '--quiet',          href: '/reference/cli#run-summary',          blurb: 'Reduce the closing summary to a bare count.' },
    { key: '--select <slug>',  href: '/reference/cli#precedence',           blurb: 'Run only the listed rule.' },
    { key: '--stdin',          href: '/reference/cli#prose-format',         blurb: 'Read source from stdin and write the rewrite to stdout.' },
    { key: '--stdin-filename', href: '/reference/cli#prose-format',         blurb: 'Treat stdin as this filename, whose extension selects Python or a notebook.' },
    { key: '--validate',       href: '/reference/cli#prose-check',          blurb: 'Confirm the would-be rewrite parses and settles, failing the run on either.' },
    { key: '--verbose',        href: '/reference/cache#hit-miss-telemetry', blurb: 'Print one line of cache hit and miss counts to stderr at the end of the run.' }
  ],
  'config-key': [
    { key: 'allow',                       href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule exemption list, modules for `bare-imports` and names for `reassigned-constants`.' },
    { key: 'allow-pattern',               href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule glob exempting matching names from a lint.' },
    { key: 'cache.enabled',               href: '/reference/cache#configuration',                 blurb: 'Turn the per-user cache on or off.' },
    { key: 'cache.max-entries',           href: '/reference/cache#configuration',                 blurb: 'The entry count eviction reduces the cache directory to.' },
    { key: 'cache.max-size-mib',          href: '/reference/cache#configuration',                 blurb: 'The size in MiB eviction reduces the cache directory to.' },
    { key: 'code-line-length',            href: '/reference/configuration#top-level-keys',        blurb: 'The line budget for code lines.' },
    { key: 'docstring-line-length',       href: '/reference/configuration#docstring-budgets',     blurb: 'The line budget for docstring prose.' },
    { key: 'docstring-structured-policy', href: '/reference/configuration#docstring-budgets',     blurb: 'Which budget a structured docstring section wraps to.' },
    { key: 'drop-duplicates',             href: '/reference/configuration#per-rule-facets',       blurb: 'Drop an import that rebinds a name an earlier import already bound to the same source.' },
    { key: 'drop-unreferenced',           href: '/reference/configuration#per-rule-facets',       blurb: 'Drop an import binding nothing references, reporting it instead inside a package `__init__.py`.' },
    { key: 'enabled',                     href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule switch, the bare bool in `[rules]`.' },
    { key: 'exempt-aliased',              href: '/reference/configuration#per-rule-facets',       blurb: 'Exempt every aliased bare import from `bare-imports`.' },
    { key: 'explode',                     href: '/reference/configuration#per-rule-facets',       blurb: 'Explode a collection that overflows the budget or exceeds its entry cap to one entry per line.' },
    { key: 'group-methods',               href: '/reference/configuration#per-rule-facets',       blurb: 'Group methods into dunders, properties, privates, and publics before sorting.' },
    { key: 'group-subcategories',         href: '/reference/configuration#per-rule-facets',       blurb: 'Cluster each band by subcategory before sorting by name.' },
    { key: 'import-line-length',          href: '/reference/configuration#top-level-keys',        blurb: 'The line budget for a `from` import, falling back to `code-line-length`.' },
    { key: 'imports.first-party',         href: '/reference/configuration#imports',               blurb: 'Package names whose imports sort into the local-package group.' },
    { key: 'keep-multiline-literals',     href: '/reference/configuration#per-rule-facets',       blurb: 'Keep a literal written as a column of two or more entries, one per line.' },
    { key: 'max-args',                    href: '/reference/configuration#per-rule-facets',       blurb: 'Argument count above which a call explodes to one `name=value` per line.' },
    { key: 'max-atomics',                 href: '/reference/configuration#per-rule-facets',       blurb: 'Atomic entries one packed row of an expanded collection may carry.' },
    { key: 'max-attributes',              href: '/reference/configuration#per-rule-facets',       blurb: 'Attribute count at or below which an unaliased bare import is reported.' },
    { key: 'max-dict-entries',            href: '/reference/configuration#per-rule-facets',       blurb: 'Entry count above which a dict explodes, whatever its width.' },
    { key: 'max-links',                   href: '/reference/configuration#per-rule-facets',       blurb: 'Link count above which a method chain breaks to one link per line.' },
    { key: 'max-params',                  href: '/reference/configuration#per-rule-facets',       blurb: 'Parameter count above which a signature explodes to one per line.' },
    { key: 'max-shift',                   href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule padding limit for an alignment run or a hanging chain.' },
    { key: 'max-tiers',                   href: '/reference/configuration#per-rule-facets',       blurb: 'Cap on the evaluation tiers that get their own sub-band.' },
    { key: 'merge-members',               href: '/reference/configuration#per-rule-facets',       blurb: 'Merge every `from` import of one module into a single statement.' },
    { key: 'overrides.paths',             href: '/reference/configuration#per-pattern-overrides', blurb: 'Glob list naming the files an override entry applies its partial config to.' },
    { key: 'report-unstable-output',      href: '/reference/configuration#top-level-keys',        blurb: 'Report a rewrite a second run would change as a defect in *Prose*.' },
    { key: 'rewrite-generics',            href: '/reference/configuration#per-rule-facets',       blurb: 'Convert a `typing` generic to the builtin PEP 585 gave it.' },
    { key: 'rewrite-identity',            href: '/reference/configuration#per-rule-facets',       blurb: 'Rewrite a `None` test to `is`, and flag a test against `True` or `False`.' },
    { key: 'rewrite-negation',            href: '/reference/configuration#per-rule-facets',       blurb: 'Fold a leading `not` into the `in` or `is` it negates.' },
    { key: 'rewrite-operand-order',       href: '/reference/configuration#per-rule-facets',       blurb: 'Swap the operands of a comparison whose constant side comes first, so the variable comes first.' },
    { key: 'rewrite-percent',             href: '/reference/configuration#per-rule-facets',       blurb: 'Convert printf-style `%` interpolation to an f-string.' },
    { key: 'rewrite-str-format',          href: '/reference/configuration#per-rule-facets',       blurb: 'Convert a `str.format()` call to an f-string.' },
    { key: 'rewrite-unions',              href: '/reference/configuration#per-rule-facets',       blurb: 'Rewrite `Optional` and `Union` to the PEP 604 pipe form.' },
    { key: 'sort-definitions',            href: '/reference/configuration#per-rule-facets',       blurb: 'Sort class and function definitions, keeping each below what it names.' },
    { key: 'sort-dict-keys',              href: '/reference/configuration#per-rule-facets',       blurb: 'Sort the entries of a dict literal, or `false` to keep the order written.' },
    { key: 'sort-docstring-entries',      href: '/reference/configuration#per-rule-facets',       blurb: 'Sort the `name: description` entries within a docstring section.' },
    { key: 'sort-dunder-lists',           href: '/reference/configuration#per-rule-facets',       blurb: 'Sort the string items inside `__all__` and `__slots__`.' },
    { key: 'split-multi-module',          href: '/reference/configuration#per-rule-facets',       blurb: 'Break a comma-joined `import a, b` into one statement per module.' },
    { key: 'suggest-string-splits',       href: '/reference/configuration#per-rule-facets',       blurb: 'Suggest the adjacent-literal form for an over-budget line a string split can shorten.' },
    { key: 'target-version',              href: '/reference/configuration#top-level-keys',        blurb: 'The Python version the project runs on, read by the version-gated rules.' },
    { key: 'unify-numerics',              href: '/reference/configuration#per-rule-facets',       blurb: 'Uppercase hex digits and lowercase the radix marker, exponent, and `j` suffix.' },
    { key: 'unify-prefixes',              href: '/reference/configuration#per-rule-facets',       blurb: 'Lowercase a string prefix and drop the no-op `u`.' },
    { key: 'unify-quotes',                href: '/reference/configuration#per-rule-facets',       blurb: 'Rewrite a non-docstring string to `"` quotes, removing an escape the quote makes unnecessary.' },
    { key: 'wrap-dict-entries',           href: '/reference/configuration#per-rule-facets',       blurb: 'Break an over-wide `key: value` at its `:` and hang the value beneath.' }
  ],
  'exit-code': [
    { key: '0', href: '/reference/exit-codes', blurb: 'Clean run, nothing pending.' },
    { key: '1', href: '/reference/exit-codes', blurb: 'A rewrite is pending under `check` or `format --diff`.' },
    { key: '2', href: '/reference/exit-codes', blurb: 'Lint findings reported.' },
    { key: '3', href: '/reference/exit-codes', blurb: 'Parse failure on at least one file.' },
    { key: '4', href: '/reference/exit-codes', blurb: 'Invalid command line or configuration.' }
  ],
  'output-format': [
    { key: 'github', href: '/reference/output-formats#github', blurb: 'Workflow-command annotations on the PR diff.' },
    { key: 'json',   href: '/reference/output-formats#json',   blurb: 'NDJSON diagnostics for editors and tooling.' },
    { key: 'sarif',  href: '/reference/output-formats#sarif',  blurb: 'SARIF document for GitHub Code Scanning.' },
    { key: 'text',   href: '/reference/output-formats#text',   blurb: 'Human-readable snippets, the default.' }
  ],
  'subcommand': [
    { key: 'prose cache clean',   href: '/reference/cache#prose-cache-clean',   blurb: 'Delete every cache entry and print the bytes freed.' },
    { key: 'prose cache compact', href: '/reference/cache#prose-cache-compact', blurb: 'Evict the oldest entries until the cache fits its configured caps.' },
    { key: 'prose cache info',    href: '/reference/cache#prose-cache-info',    blurb: 'Print the cache path, entry count, byte total, and oldest and newest mtimes.' },
    { key: 'prose check',         href: '/reference/cli#prose-check',           blurb: 'Report what would change without rewriting, exiting non-zero when a rewrite is pending.' },
    { key: 'prose completions',   href: '/reference/cli#prose-completions',     blurb: 'Print a shell-completion script for the named shell.' },
    { key: 'prose format',        href: '/reference/cli#prose-format',          blurb: 'Rewrite files in place.' },
    { key: 'prose rules',         href: '/reference/cli#prose-rules',           blurb: 'List every registered rule in pipeline order.' },
    { key: 'prose schema',        href: '/reference/cli#prose-schema',          blurb: 'Print the configuration\'s JSON Schema, every key with its type, default, and range.' },
    { key: 'prose server',        href: '/reference/cli#prose-server',          blurb: 'Serve format-on-save and live diagnostics over the language-server protocol.' }
  ],
  'suppression': DIRECTIVES.map(d => ({
    key   : d.form,
    href  : directiveHref(d.scope),
    blurb : d.blurb
  }))
}

export function groupByDomain(tokens: readonly Token[]): [Domain, Token[]][] {
  return [...Map.groupBy(tokens, t => t.domain).entries()]
    .toSorted(([a], [b]) => a.localeCompare(b))
    .map(([d, bucket]) => [d, bucket.toSorted((a, b) => a.sort.localeCompare(b.sort))])
}

export function stripPrefix(s: string): string {
  return s.replace(/^[#\-\s]+/, '').replace(/^(prose|fmt|yapf)\s*:?\s*/i, '').toLowerCase()
}
