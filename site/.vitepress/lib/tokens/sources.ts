import type { InlineNode }      from '../markdown/inline-nodes'
import { DIRECTIVES }           from '../suppression/directives'
import { directiveHref }        from '../suppression/scopes'

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
    { key: '--color',          href: '/reference/cli#global-flag',          blurb: 'Color-output mode for human-readable output.' },
    { key: '--diff',           href: '/reference/cli#prose-format',         blurb: 'Print a unified diff without rewriting the source.' },
    { key: '--ignore <slug>',  href: '/reference/cli#precedence',           blurb: 'Subtract the listed rule from the active set.' },
    { key: '--no-cache',       href: '/reference/cache',                    blurb: 'Bypass the user-level cache for the single invocation.' },
    { key: '--output-format',  href: '/reference/cli#prose-format',         blurb: 'Pick the diagnostic shape (`text` / `json` / `github` / `sarif`).' },
    { key: '--quiet',          href: '/reference/cli#run-summary',          blurb: 'Reduce the closing summary to a bare count line.' },
    { key: '--select <slug>',  href: '/reference/cli#precedence',           blurb: 'Restrict the run to the listed rule.' },
    { key: '--stdin',          href: '/reference/cli#prose-format',         blurb: 'Read source from stdin, write the rewrite to stdout.' },
    { key: '--stdin-filename', href: '/reference/cli#prose-format',         blurb: 'Treat stdin as this filename, its extension selecting Python or a notebook.' },
    { key: '--validate',       href: '/reference/cli#prose-check',          blurb: 'Confirm the would-be rewrite re-parses, surfacing an unparseable rule output.' },
    { key: '--verbose',        href: '/reference/cache#hit-miss-telemetry', blurb: 'Print a one-line cache summary to stderr at the end of the run.' }
  ],
  'config-key': [
    { key: 'allow',                       href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule exemption list, modules for `bare-imports` and names for `reassigned-constants`.' },
    { key: 'allow-pattern',               href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule regex exempting matching names from a lint.' },
    { key: 'cache.enabled',               href: '/reference/cache#configuration',                 blurb: 'Toggle the user-level cache globally.' },
    { key: 'cache.max-size-mib',          href: '/reference/cache#configuration',                 blurb: 'LRU eviction cap on the cache directory.' },
    { key: 'code-line-length',            href: '/reference/configuration#top-level-keys',        blurb: 'Maximum column budget for code lines.' },
    { key: 'docstring-line-length',       href: '/reference/configuration#docstring-budgets',     blurb: 'Maximum column budget for docstring prose.' },
    { key: 'docstring-structured-policy', href: '/reference/configuration#docstring-budgets',     blurb: 'Budget policy for docstring structured sections.' },
    { key: 'drop-duplicates',             href: '/reference/configuration#per-rule-facets',       blurb: 'Drop an import rebinding a name an earlier import already bound to the same source.' },
    { key: 'drop-unreferenced',           href: '/reference/configuration#per-rule-facets',       blurb: 'Drop an import binding nothing references, reporting it instead inside a package `__init__.py`.' },
    { key: 'enabled',                     href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule toggle, the bare bool in `[rules]`.' },
    { key: 'exempt-aliased',              href: '/reference/configuration#per-rule-facets',       blurb: 'Spare every aliased bare import from `bare-imports`.' },
    { key: 'explode',                     href: '/reference/configuration#per-rule-facets',       blurb: 'Expand an overflowing or over-count collection to one entry per line.' },
    { key: 'group-methods',               href: '/reference/configuration#per-rule-facets',       blurb: 'Group methods into dunders, properties, privates, and publics before sorting.' },
    { key: 'group-subcategories',         href: '/reference/configuration#per-rule-facets',       blurb: 'Cluster each band by subcategory before sorting by name.' },
    { key: 'import-line-length',          href: '/reference/configuration#top-level-keys',        blurb: 'Import-wrap column budget, falls back to `code-line-length`.' },
    { key: 'imports.first-party',         href: '/reference/configuration#imports',               blurb: 'Package names lifted into the local-package import group.' },
    { key: 'keep-multiline-literals',     href: '/reference/configuration#per-rule-facets',       blurb: 'Hold a literal laid out as a flush column of two or more entries.' },
    { key: 'max-args',                    href: '/reference/configuration#per-rule-facets',       blurb: 'Argument count above which a call explodes to one keyword per line.' },
    { key: 'max-atomics',                 href: '/reference/configuration#per-rule-facets',       blurb: 'Inline cap on a short collection of atomic literals.' },
    { key: 'max-attributes',              href: '/reference/configuration#per-rule-facets',       blurb: 'Attribute count at or below which an unaliased bare import is flagged.' },
    { key: 'max-dict-entries',            href: '/reference/configuration#per-rule-facets',       blurb: 'Entry count above which a dict expands, whatever its width.' },
    { key: 'max-links',                   href: '/reference/configuration#per-rule-facets',       blurb: 'Link count above which a method chain breaks to one link per line.' },
    { key: 'max-params',                  href: '/reference/configuration#per-rule-facets',       blurb: 'Parameter count above which a signature expands to one per line.' },
    { key: 'max-shift',                   href: '/reference/configuration#per-rule-facets',       blurb: 'Per-rule shift budget for an alignment run or a chain hang.' },
    { key: 'max-tiers',                   href: '/reference/configuration#per-rule-facets',       blurb: 'Cap the evaluation tiers that open their own sub-band.' },
    { key: 'merge-members',               href: '/reference/configuration#per-rule-facets',       blurb: 'Gather every from-import of one module onto a single statement.' },
    { key: 'overrides.paths',             href: '/reference/configuration#per-pattern-overrides', blurb: 'Glob list selecting the files an override entry applies its partial config to.' },
    { key: 'rewrite-generics',            href: '/reference/configuration#per-rule-facets',       blurb: 'Convert a `typing` generic to the builtin PEP 585 gave it.' },
    { key: 'rewrite-identity',            href: '/reference/configuration#per-rule-facets',       blurb: 'Rewrite a `None` test to `is`, and flag a test against `True` or `False`.' },
    { key: 'rewrite-negation',            href: '/reference/configuration#per-rule-facets',       blurb: 'Fold a leading `not` into the `in` or `is` it negates.' },
    { key: 'rewrite-operand-order',       href: '/reference/configuration#per-rule-facets',       blurb: 'Flip a comparison whose constant side leads, so the variable leads.' },
    { key: 'rewrite-unions',              href: '/reference/configuration#per-rule-facets',       blurb: 'Rewrite `Optional` and `Union` to the PEP 604 pipe form.' },
    { key: 'sort-definitions',            href: '/reference/configuration#per-rule-facets',       blurb: 'Reorder class and function definitions, holding each behind what it names.' },
    { key: 'sort-dict-keys',              href: '/reference/configuration#per-rule-facets',       blurb: 'Reorder the keyed entries of a dict literal, off to hold the authored order.' },
    { key: 'sort-docstring-entries',      href: '/reference/configuration#per-rule-facets',       blurb: 'Reorder `name: description` entries within a docstring section.' },
    { key: 'sort-dunder-lists',           href: '/reference/configuration#per-rule-facets',       blurb: 'Reorder the string items inside `__all__` and `__slots__`.' },
    { key: 'split-multi-module',          href: '/reference/configuration#per-rule-facets',       blurb: 'Break a comma-joined `import a, b` into one statement per module.' },
    { key: 'suggest-string-splits',       href: '/reference/configuration#per-rule-facets',       blurb: 'Offer the adjacent-literal form for an over-budget splittable string.' },
    { key: 'target-version',              href: '/reference/configuration#top-level-keys',        blurb: 'Python version the parser reads against.' },
    { key: 'unify-numerics',              href: '/reference/configuration#per-rule-facets',       blurb: 'Uppercase hex digits while the radix marker, exponent, and `j` suffix lowercase.' },
    { key: 'unify-prefixes',              href: '/reference/configuration#per-rule-facets',       blurb: 'Lowercase a string prefix and drop the no-op `u`.' },
    { key: 'unify-quotes',                href: '/reference/configuration#per-rule-facets',       blurb: 'Settle a non-docstring string on `"`, shedding an escape the quote does not need.' },
    { key: 'wrap-dict-entries',           href: '/reference/configuration#per-rule-facets',       blurb: 'Break an over-wide `key: value` at its `:` and hang the value beneath.' }
  ],
  'exit-code': [
    { key: '0', href: '/reference/exit-codes', blurb: 'Clean run, every rewrite applied.' },
    { key: '1', href: '/reference/exit-codes', blurb: 'Pending rewrites under check.' },
    { key: '2', href: '/reference/exit-codes', blurb: 'Lint diagnostics emitted.' },
    { key: '3', href: '/reference/exit-codes', blurb: 'Parse failure on at least one file.' },
    { key: '4', href: '/reference/exit-codes', blurb: 'Invalid CLI invocation or configuration.' }
  ],
  'output-format': [
    { key: 'github', href: '/reference/output-formats#github', blurb: 'Workflow-command annotations for inline PR review.' },
    { key: 'json',   href: '/reference/output-formats#json',   blurb: 'LSP-style structured diagnostics.' },
    { key: 'sarif',  href: '/reference/output-formats#sarif',  blurb: 'GitHub Code Scanning upload format.' },
    { key: 'text',   href: '/reference/output-formats#text',   blurb: 'Default human-readable output.' }
  ],
  'subcommand': [
    { key: 'prose cache clean',   href: '/reference/cache#prose-cache-clean',   blurb: 'Clear every cached entry and report the freed bytes.' },
    { key: 'prose cache compact', href: '/reference/cache#prose-cache-compact', blurb: 'Evict oldest entries until the configured size cap is met.' },
    { key: 'prose cache info',    href: '/reference/cache#prose-cache-info',    blurb: 'Print cache path, entry count, byte total, and mtimes.' },
    { key: 'prose check',         href: '/reference/cli#prose-check',           blurb: 'Verify without rewriting, resolving to a non-zero exit code when any rewrite pends.' },
    { key: 'prose completions',   href: '/reference/cli#prose-completions',     blurb: 'Emit shell-completion scripts for the active shell.' },
    { key: 'prose format',        href: '/reference/cli#prose-format',          blurb: 'Apply every pending rewrite in place.' },
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
