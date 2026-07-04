import { applyCompletedEdits }                      from './typing-demo-buffer'
import type { TypingDemoEntry, TypingDemoResetRow } from './typing-demo-buffer'

export const SOURCE = `from pathlib import Path
from dataclasses import dataclass
@dataclass
class Config:
    """Connection knobs read at startup from the project's pyproject.toml, with command-line overrides applied last."""
    timeout: int | None = None
    name: str = "service"
    debug: bool = False
    def build_request(self, db: str, fully_qualified_endpoint_url: str, body: dict, timeout: int = 30, headers: dict | None = None,) -> dict:
        """Build a configured request with per-call overrides."""
        headers_map = {"x-api-key": "secret", "accept": "application/json", "x-request-correlation-id": "abc-123"}
        base_url = "https://example.com"
        return {"url": f"{base_url}/{fully_qualified_endpoint_url}"}
`

// Demo play order, not alphabetical. Each morph state runs a cumulative prefix.
export const RULES = [
  'align-equals',
  'align-colons',
  'align-imports',
  'signature-layout',
  'alphabetize',
  'docstring-expand',
  'docstring-wrap',
  'blank-lines',
  'collection-layout'
] as const

const RULE_COLUMN = Math.max(...RULES.map(rule => rule.length))
const RULES_NOTE  = "# Rules are on by default.\n# All 'true' values are just for show."

function ruleAnchor(slug: string): string {
  return `${slug.padEnd(RULE_COLUMN)} = `
}

export const PRELUDE = `code-line-length      = 88
docstring-line-length = 76
target-version        = "3.13"

${RULES_NOTE}
[rules]
${RULES.map(slug => `${ruleAnchor(slug)}false`).join('\n')}
`

interface TailValues {
  alignColons         ?: string
  alignEquals         ?: number
  docstringLineLength  : number
}

function tail({ alignColons, alignEquals, docstringLineLength }: TailValues): string {
  const base = `code-line-length      = 88
docstring-line-length = ${docstringLineLength}
target-version        = "3.13"
`
  const rules = [
    alignEquals !== undefined ? `align-equals = { max-shift = ${alignEquals} }` : null,
    alignColons !== undefined ? `align-colons = ${alignColons}`                 : null
  ].filter(Boolean)
  return rules.length === 0
    ? base
    : `${base}\n[rules]\n${rules.join('\n')}\n`
}

export const ENTRIES: readonly TypingDemoEntry[] = [
  ...RULES.map((slug): TypingDemoEntry => ({
    anchor : ruleAnchor(slug),
    from   : 'false',
    kind   : 'edit',
    slug,
    to     : 'true'
  })),
  {
    anchor : ruleAnchor('align-equals'),
    from   : 'true',
    kind   : 'edit',
    slug   : 'max-shift',
    tail   : tail({ alignEquals: 6, docstringLineLength: 76 }),
    to     : '{ max-shift = 6 }'
  },
  {
    anchor : ruleAnchor('align-colons'),
    from   : 'true',
    kind   : 'edit',
    slug   : 'max-shift',
    tail   : tail({ alignColons: '{ max-shift = false }', alignEquals: 6, docstringLineLength: 76 }),
    to     : '{ max-shift = false }'
  },
  {
    anchor : 'docstring-line-length = ',
    from   : '76',
    kind   : 'edit',
    slug   : 'docstring-line-length',
    tail   : tail({ alignColons: '{ max-shift = false }', alignEquals: 6, docstringLineLength: 60 }),
    to     : '60'
  }
]

function buildResetRows(): TypingDemoResetRow[] {
  const rows = new Map<string, TypingDemoResetRow>()
  for (const entry of ENTRIES) {
    const row = rows.get(entry.anchor)
    if (row) row.end = entry.to
    else rows.set(entry.anchor, { anchor: entry.anchor, end: entry.to, prelude: entry.from })
  }
  return [...rows.values()]
}

export const RESET_ROWS: readonly TypingDemoResetRow[] = buildResetRows()

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('typing demo data', () => {
    test('the source carries the dataclass under format', () => {
      expect(SOURCE).toContain('class Config:')
    })

    test.each(RULES.map(rule => ({ rule })))('the prelude lists $rule as false', ({ rule }) => {
      expect(PRELUDE).toContain(rule)
      expect(ENTRIES.some(entry => entry.slug === rule && entry.from === 'false' && entry.to === 'true')).toBe(true)
    })

    test('applying the rule toggles flips every false to true', () => {
      const toggled = applyCompletedEdits(PRELUDE, ENTRIES, RULES.length)
      for (const rule of RULES) expect(toggled).toContain(rule)
      expect(toggled).not.toContain('= false')
      expect(toggled).toContain('= true')
    })

    test('reset rows carry one entry per unique anchor', () => {
      const anchors = new Set(ENTRIES.map(entry => entry.anchor))
      expect(RESET_ROWS).toHaveLength(anchors.size)
      expect(new Set(RESET_ROWS.map(row => row.anchor))).toEqual(anchors)
    })

    test('a reset row keeps the first prelude and the last end value', () => {
      const alignRow = RESET_ROWS.find(row => row.anchor.startsWith('align-equals'))
      expect(alignRow?.prelude).toBe('false')
      expect(alignRow?.end).toBe('{ max-shift = 6 }')
    })
  })
}
