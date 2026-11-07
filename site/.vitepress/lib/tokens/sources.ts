import type { InlineNode }  from '../markdown/inline-nodes'
import { DIRECTIVES }       from '../suppression/directives'
import { directiveHref }    from '../suppression/scopes'
import type { Destination } from './destinations'

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
  blurbNodes  : InlineNode[]
  destination : Destination
  domain      : Domain
  href        : string
  key         : string
  sort        : string
}

export const DOMAIN_LABELS: Record<Domain, string> = {
  'cli-flag'      : 'CLI Flag',
  'config-key'    : 'Configuration Key',
  'exit-code'     : 'Exit Code',
  'output-format' : 'Output Format',
  'subcommand'    : 'Subcommand',
  'suppression'   : 'Suppression Directive'
}

// The domains whose blurb is the first sentence of the line `prose --help`
// prints for the entry.
export type HelpDomain = 'cli-flag' | 'subcommand'

export type StaticDomain = Exclude<Domain, 'config-key' | 'exit-code' | HelpDomain>

export const HELP_SOURCES: Record<HelpDomain, readonly Omit<TokenSource, 'blurb'>[]> = {
  'cli-flag': [
    { key: '--color',          href: '/reference/cli#global-flags' },
    { key: '--diff',           href: '/reference/cli#prose-format' },
    { key: '--ignore <slug>',  href: '/reference/cli#precedence' },
    { key: '--no-cache',       href: '/reference/cache' },
    { key: '--output-format',  href: '/reference/cli#prose-format' },
    { key: '--quiet',          href: '/reference/cli#run-summary' },
    { key: '--select <slug>',  href: '/reference/cli#precedence' },
    { key: '--stdin',          href: '/reference/cli#prose-format' },
    { key: '--stdin-filename', href: '/reference/cli#prose-format' },
    { key: '--validate',       href: '/reference/cli#prose-check' },
    { key: '--verbose',        href: '/reference/cache#hit-miss-telemetry' }
  ],
  'subcommand': [
    { key: 'prose cache clean',   href: '/reference/cache#prose-cache-clean' },
    { key: 'prose cache compact', href: '/reference/cache#prose-cache-compact' },
    { key: 'prose cache info',    href: '/reference/cache#prose-cache-info' },
    { key: 'prose check',         href: '/reference/cli#prose-check' },
    { key: 'prose completions',   href: '/reference/cli#prose-completions' },
    { key: 'prose format',        href: '/reference/cli#prose-format' },
    { key: 'prose rules',         href: '/reference/cli#prose-rules' },
    { key: 'prose schema',        href: '/reference/cli#prose-schema' },
    { key: 'prose server',        href: '/reference/cli#prose-server' }
  ]
}

export const SOURCES: Record<StaticDomain, readonly TokenSource[]> = {
  'output-format': [
    { key: 'github', href: '/reference/output-formats#github', blurb: 'Workflow-command annotations on the PR diff.' },
    { key: 'json',   href: '/reference/output-formats#json',   blurb: 'NDJSON diagnostics for editors and tooling.' },
    { key: 'sarif',  href: '/reference/output-formats#sarif',  blurb: 'SARIF document for GitHub Code Scanning.' },
    { key: 'text',   href: '/reference/output-formats#text',   blurb: 'Human-readable snippets, the default.' }
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
