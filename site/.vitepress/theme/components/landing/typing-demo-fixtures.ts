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
    def build_request(self, endpoint_url: str, body: dict, timeout: int = 30, headers: dict | None = None,) -> dict:
        """Build a configured request with per-call overrides."""
        headers_map = {"x-api-key": "secret", "accept": "application/json", "x-trace-id": "abc-123"}
        base_url = "https://example.com"
        return {"url": f"{base_url}/{endpoint_url}"}
`

export const RULES = [
  'align-equals',
  'align-colons',
  'align-imports',
  'signature-layout',
  'alphabetize',
  'no-single-line-docstrings',
  'docstring-wrap',
  'blank-lines',
  'collection-layout'
] as const

const RULE_COLUMN = Math.max(...RULES.map(rule => rule.length))
const RULES_NOTE  = "# Rules are on by default.\n# These 'true' lines are just for illustration."

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
  codeLineLength      : number
  docstringLineLength : number
  maxShift           ?: number
}

function tail({ codeLineLength, docstringLineLength, maxShift }: TailValues): string {
  const base = `code-line-length      = ${codeLineLength}
docstring-line-length = ${docstringLineLength}
target-version        = "3.13"
`
  return maxShift === undefined
    ? base
    : `${base}\n[rules]\nalign-equals = { max-shift = ${maxShift} }\n`
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
    anchor : 'code-line-length      = ',
    from   : '88',
    kind   : 'edit',
    slug   : 'code-line-length',
    tail   : tail({ codeLineLength: 100, docstringLineLength: 76 }),
    to     : '100'
  },
  {
    anchor : ruleAnchor('align-equals'),
    from   : 'true',
    kind   : 'edit',
    slug   : 'max-shift',
    tail   : tail({ codeLineLength: 100, docstringLineLength: 76, maxShift: 6 }),
    to     : '{ max-shift = 6 }'
  },
  {
    anchor : 'docstring-line-length = ',
    from   : '76',
    kind   : 'edit',
    slug   : 'docstring-line-length',
    tail   : tail({ codeLineLength: 100, docstringLineLength: 60, maxShift: 6 }),
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
