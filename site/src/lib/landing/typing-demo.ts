export interface LandingTypingDemoEditEntry {
  anchor : string
  from   : string
  kind   : 'edit'
  slug   : string
  tail  ?: string
  to     : string
}

export type LandingTypingDemoEntry = LandingTypingDemoEditEntry

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

export const ENTRIES: readonly LandingTypingDemoEntry[] = [
  ...RULES.map((slug): LandingTypingDemoEditEntry => ({
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

export interface LandingTypingDemoResetRow {
  anchor  : string
  end     : string
  prelude : string
}

function buildResetRows(): LandingTypingDemoResetRow[] {
  const rows  : LandingTypingDemoResetRow[] = []
  const index = new Map<string, number>()
  for (const entry of ENTRIES) {
    const at = index.get(entry.anchor)
    if (at === undefined) {
      index.set(entry.anchor, rows.length)
      rows.push({ anchor: entry.anchor, end: entry.to, prelude: entry.from })
    } else {
      rows[at].end = entry.to
    }
  }
  return rows
}

export const RESET_ROWS: readonly LandingTypingDemoResetRow[] = buildResetRows()
