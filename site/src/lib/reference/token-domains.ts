export const TOKEN_DOMAINS = [
  'cli-flag', 'config-key', 'exit-code', 'output-format', 'subcommand', 'suppression'
] as const

export type TokenDomain = (typeof TOKEN_DOMAINS)[number]

// Sorts punctuation-led keys under their first word character, so `--diff`
// files under D.
export const tokenSortKey = (key: string): string => key.replace(/^[^a-z0-9]+/i, '').toLowerCase()
