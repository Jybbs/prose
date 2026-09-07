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

export interface TokenSource {
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

export type StaticDomain = Exclude<Domain, 'config-key' | 'exit-code'>

export const SOURCES: Record<StaticDomain, readonly TokenSource[]> = {
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
