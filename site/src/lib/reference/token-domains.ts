export const TOKEN_DOMAINS = [
  'cli-flag', 'config-key', 'exit-code', 'output-format', 'subcommand', 'suppression'
] as const

export type TokenDomain = (typeof TOKEN_DOMAINS)[number]

// Sorts punctuation-led keys under their first word character, so `--diff`
// files under D.
export const tokenSortKey = (key: string): string => key.replace(/^[^a-z0-9]+/i, '').toLowerCase()

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('tokenSortKey', () => {
    test.each([
      { name: 'strips a leading double dash',    key: '--diff',     expected: 'diff'     },
      { name: 'lowercases the result',           key: 'Subcommand', expected: 'subcommand' },
      { name: 'leaves a bare word untouched',    key: 'config',     expected: 'config'   },
      { name: 'strips leading punctuation only', key: ':=walrus',   expected: 'walrus'   }
    ])('$name', ({ key, expected }) => {
      expect(tokenSortKey(key)).toBe(expected)
    })

    test('rosters every documented domain', () => {
      expect(TOKEN_DOMAINS).toContain('cli-flag')
      expect(TOKEN_DOMAINS).toContain('suppression')
    })
  })
}
