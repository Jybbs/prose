import { formatHex, interpolate } from 'culori'
import * as fc                    from 'fast-check'

const TOKENS = {
  'family-alignment'     : 'var(--prose-palette-eureka)',
  'family-cli'           : 'var(--prose-palette-ube-night)',
  'family-docs'          : 'var(--prose-palette-celadon)',
  'family-engine'        : 'var(--prose-palette-ube)',
  'family-formatting'    : 'var(--prose-palette-heath)',
  'family-layout'        : 'var(--prose-palette-toronto)',
  'family-lint'          : 'var(--prose-palette-apricot)',
  'family-ordering'      : 'var(--prose-palette-chambray)',
  'palette-apricot'      : '#e8876f',
  'palette-casper'       : '#adbdcd',
  'palette-celadon'      : '#8cc5a3',
  'palette-chambray'     : '#7db3e0',
  'palette-champagne'    : '#f0e9bc',
  'palette-dexter'       : '#6db0b5',
  'palette-eureka'       : '#e8c840',
  'palette-grams-hair'   : '#f6f8fa',
  'palette-heath'        : '#c08597',
  'palette-oat'          : '#cdbda5',
  'palette-rainee'       : '#b8c8a8',
  'palette-toronto'      : '#5069ad',
  'palette-ube'          : '#8a80cb',
  'palette-ube-deep'     : 'color-mix(in oklch, var(--prose-palette-ube), black 22%)',
  'palette-ube-mid'      : 'color-mix(in oklch, var(--prose-palette-ube), white 18%)',
  'palette-ube-night'    : 'color-mix(in oklch, var(--prose-palette-ube), black 45%)',
  'palette-ube-pale'     : 'color-mix(in oklch, var(--prose-palette-ube), white 36%)',
  'palette-ube-wash'     : 'color-mix(in oklch, var(--prose-palette-ube), white 82%)',
  'palette-whiskey'      : '#d4a574',
  'palette-woodsmoke'    : '#17171b',
  'role-accent'          : 'var(--prose-palette-chambray)',
  'role-error'           : 'var(--prose-palette-apricot)',
  'role-link-hover'      : 'var(--prose-palette-ube-deep)',
  'role-warning'         : 'var(--prose-palette-eureka)',
  'section-integrations' : 'var(--prose-palette-rainee)',
  'section-primitives'   : 'var(--prose-palette-dexter)',
  'section-reference'    : 'var(--prose-palette-casper)',
  'section-usage'        : 'var(--prose-palette-oat)'
} as const satisfies Record<string, string>

export type TokenName = keyof typeof TOKENS

const MIX = /^color-mix\(in oklch, var\(--prose-([\w-]+)\), (black|white) (\d+)%\)$/

// Evaluates a token to a concrete color, following `var()` aliases and
// computing `color-mix()` blends, the same operation CSS performs for the
// browser.
export function resolveColor(name: TokenName): string {
  return lookup(name)
}

// The string-typed path the recursion and computed-key callers take to reach a
// token the `TokenName` union cannot name.
function lookup(name: string): string {
  const value = (TOKENS as Record<string, string>)[name] ?? ''
  const alias = value.match(/^var\(--prose-([\w-]+)\)$/)
  if (alias !== null) return lookup(alias[1])
  const mix = value.match(MIX)
  if (mix === null) return value
  const [, base, toward, share] = mix
  return formatHex(interpolate([lookup(base), toward], 'oklch')(Number(share) / 100))
}

export function tokensToCss(): string {
  const lines = Object.entries(TOKENS).map(([name, value]) => `  --prose-${name}: ${value};`)
  return `:root {\n${lines.join('\n')}\n}\n`
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const HEX = /^#[0-9a-f]{6}$/

  describe('resolveColor', () => {
    test.each([
      { name: 'returns a plain hex token unchanged', token: 'palette-apricot',  expected: '#e8876f' },
      { name: 'follows a var() alias to its hex',    token: 'role-accent',      expected: '#7db3e0' },
      { name: 'follows a family alias to its hex',   token: 'family-alignment', expected: '#e8c840' }
    ] as const)('$name', ({ token, expected }) => {
      expect(resolveColor(token)).toBe(expected)
    })

    test.each([
      { name: 'blends a deep mix toward black',     token: 'palette-ube-deep' },
      { name: 'blends a pale mix toward white',     token: 'palette-ube-pale' },
      { name: 'resolves a role that aliases a mix', token: 'role-link-hover'  }
    ] as const)('$name to a concrete hex', ({ token }) => {
      expect(resolveColor(token)).toMatch(HEX)
    })
  })

  describe('lookup', () => {
    test('returns empty string for an unknown token', () => {
      expect(lookup('no-such-token')).toBe('')
    })

    test('returns empty string for any absent key', () => {
      fc.assert(fc.property(fc.string(), (suffix) => {
        expect(lookup(`absent-${suffix}-token`)).toBe('')
      }))
    })
  })

  describe('tokensToCss', () => {
    const css = tokensToCss()

    test('wraps the declarations in a :root block', () => {
      expect(css.startsWith(':root {\n')).toBe(true)
      expect(css.endsWith('}\n')).toBe(true)
    })

    test.each([
      { name: 'emits a plain hex declaration',  needle: '--prose-palette-apricot: #e8876f;'                                      },
      { name: 'emits an alias declaration',     needle: '--prose-role-accent: var(--prose-palette-chambray);'                    },
      { name: 'emits a color-mix declaration',  needle: '--prose-palette-ube-deep: color-mix(in oklch, var(--prose-palette-ube), black 22%);' }
    ])('$name', ({ needle }) => {
      expect(css).toContain(needle)
    })
  })
}
