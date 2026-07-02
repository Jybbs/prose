export const TOKEN_DOMAINS = [
  'cli-flag', 'config-key', 'exit-code', 'output-format', 'subcommand', 'suppression'
] as const

export type TokenDomain = (typeof TOKEN_DOMAINS)[number]

// The editorial singular label each index section and detail banner renders,
// which no mechanical casing of the domain slug produces.
export const DOMAIN_LABELS: Record<TokenDomain, string> = {
  'cli-flag'      : 'CLI Flag',
  'config-key'    : 'Configuration Key',
  'exit-code'     : 'Exit Code',
  'output-format' : 'Output Format',
  'subcommand'    : 'Subcommand',
  'suppression'   : 'Suppression Directive'
}

// Sorts punctuation-led keys under their first word character, so `--diff`
// files under D.
export const tokenSortKey = (key: string): string => key.replace(/^[^a-z0-9]+/i, '').toLowerCase()
