export type Domain =
  | 'cli-flag'
  | 'config-key'
  | 'exit-code'
  | 'output-format'
  | 'subcommand'
  | 'suppression'

interface DomainMeta {
  accent : string
  label  : string
  pip    : string
  short  : string
}

interface TokenSource {
  blurb : string
  href  : string
  key   : string
}

export interface Token {
  blurbHtml : string
  domain    : Domain
  href      : string
  key       : string
  sort      : string
}

export const DOMAIN_META: Record<Domain, DomainMeta> = {
  'cli-flag'      : { accent : 'var(--prose-c-ube)',      label : 'CLI Flag',              pip : 'F', short : 'flag'      },
  'config-key'    : { accent : 'var(--prose-c-celadon)',  label : 'Configuration Key',     pip : 'K', short : 'config'    },
  'exit-code'     : { accent : 'var(--prose-c-chambray)', label : 'Exit Code',             pip : 'E', short : 'exit'      },
  'output-format' : { accent : 'var(--prose-c-whiskey)',  label : 'Output Format',         pip : 'O', short : 'output'    },
  'subcommand'    : { accent : 'var(--prose-c-eureka)',   label : 'Subcommand',            pip : 'S', short : 'cmd'       },
  'suppression'   : { accent : 'var(--prose-c-apricot)',  label : 'Suppression Directive', pip : 'D', short : 'directive' }
}

export const SOURCES: Record<Domain, readonly TokenSource[]> = {
  'cli-flag': [
    { key: '--color',         href: '/reference/cli#global-flag',          blurb: 'Color-output mode for human-readable output.' },
    { key: '--diff',          href: '/reference/cli#prose-format',         blurb: 'Print a unified diff without rewriting the source.' },
    { key: '--ignore <slug>', href: '/reference/cli#precedence',           blurb: 'Subtract the listed rule from the active set.' },
    { key: '--no-cache',      href: '/reference/cache',                    blurb: 'Bypass the user-level cache for the single invocation.' },
    { key: '--output-format', href: '/reference/cli#prose-format',         blurb: 'Pick the diagnostic shape (`text` / `json` / `github` / `sarif`).' },
    { key: '--quiet',         href: '/reference/cli#run-summary',          blurb: 'Reduce the closing summary to a bare count line.' },
    { key: '--select <slug>', href: '/reference/cli#precedence',           blurb: 'Restrict the run to the listed rule.' },
    { key: '--stdin',         href: '/reference/cli#prose-format',         blurb: 'Read source from stdin, write the rewrite to stdout.' },
    { key: '--verbose',       href: '/reference/cache#hit-miss-telemetry', blurb: 'Print a one-line cache summary to stderr at the end of the run.' }
  ],
  'config-key': [
    { key: 'cache.enabled',               href: '/reference/cache#configuration',             blurb: 'Toggle the user-level cache globally.' },
    { key: 'cache.max-size-mib',          href: '/reference/cache#configuration',             blurb: 'LRU eviction cap on the cache directory.' },
    { key: 'code-line-length',            href: '/reference/configuration#top-level-keys',    blurb: 'Maximum column budget for code lines.' },
    { key: 'docstring-line-length',       href: '/reference/configuration#docstring-budgets', blurb: 'Maximum column budget for docstring prose.' },
    { key: 'docstring-structured-policy', href: '/reference/configuration#docstring-budgets', blurb: 'Budget policy for docstring structured sections.' },
    { key: 'enabled',                     href: '/reference/configuration#per-rule-knobs',    blurb: 'Per-rule toggle inside `[tool.prose.rules.<slug>]`.' },
    { key: 'imports.first-party',         href: '/reference/configuration#imports',           blurb: 'Package names lifted into the local-package import group.' },
    { key: 'max-shift',                   href: '/reference/configuration#per-rule-knobs',    blurb: 'Per-rule alignment-shift bound.' },
    { key: 'max-shift-policy',            href: '/reference/configuration#per-rule-knobs',    blurb: 'Fallback (`split` / `drop`) when the widest member overflows `max-shift`.' },
    { key: 'target-version',              href: '/reference/configuration#top-level-keys',    blurb: 'Python version the parser reads against.' }
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
    { key: 'prose format',        href: '/reference/cli#prose-format',          blurb: 'Apply every pending rewrite in place.' }
  ],
  'suppression': [
    { key: '# yapf: disable',         href: '/reference/suppression-directives#block-markers',                   blurb: 'Yapf alias for `# fmt: off`.' },
    { key: '# yapf: enable',          href: '/reference/suppression-directives#block-markers',                   blurb: 'Yapf alias for `# fmt: on`.' },
    { key: '# prose: ignore[<slug>]', href: '/reference/suppression-directives#line-markers',                    blurb: 'Per-line lint suppression for the listed rule.' },
    { key: '# prose: keep',           href: '/reference/suppression-directives#dict-literal-order-preservation', blurb: 'Preserve the authored shape against rewrites.' },
    { key: '# fmt: off',              href: '/reference/suppression-directives#block-markers',                   blurb: 'Block-format suppression open.' },
    { key: '# fmt: on',               href: '/reference/suppression-directives#block-markers',                   blurb: 'Block-format suppression close.' },
    { key: '# fmt: skip',             href: '/reference/suppression-directives#line-markers',                    blurb: 'Single-line format suppression.' }
  ]
}

export function stripPrefix(s: string): string {
  return s.replace(/^[#\-\s]+/, '').replace(/^(prose|fmt|yapf)\s*:?\s*/i, '').toLowerCase()
}

export function sortedTokens(tokens: readonly Token[], mode: 'key' | 'domain' = 'key'): Token[] {
  const flat = [...tokens]
  if (mode === 'domain') {
    return flat.sort((a, b) =>
      a.domain.localeCompare(b.domain) || a.sort.localeCompare(b.sort))
  }
  return flat.sort((a, b) => a.sort.localeCompare(b.sort))
}

export function groupByDomain(tokens: readonly Token[]): [Domain, Token[]][] {
  return [...Map.groupBy(tokens, t => t.domain).entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([d, bucket]) => [d, bucket.sort((a, b) => a.sort.localeCompare(b.sort))])
}
