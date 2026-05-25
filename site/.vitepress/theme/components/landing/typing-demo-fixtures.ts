export interface LandingTypingDemoAppendEntry {
  block : string
  kind  : 'append'
  slug  : string
}

export interface LandingTypingDemoEditEntry {
  anchor : string
  from   : string
  kind   : 'edit'
  slug   : string
  tail   : string
  to     : string
}

export type LandingTypingDemoEntry =
  | LandingTypingDemoAppendEntry
  | LandingTypingDemoEditEntry

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

export const PRELUDE = `[tool.prose]
code-line-length      = 88
docstring-line-length = 76
target-version        = "3.13"

`

interface TailValues {
  codeLineLength      : number
  docstringLineLength : number
  maxShift           ?: number
}

function tail({ codeLineLength, docstringLineLength, maxShift }: TailValues): string {
  const base = `[tool.prose]
code-line-length      = ${codeLineLength}
docstring-line-length = ${docstringLineLength}
target-version        = "3.13"
`
  return maxShift === undefined
    ? base
    : `${base}\n[tool.prose.rules.align-equals]\nmax-shift = ${maxShift}\n`
}

export const ENTRIES: readonly LandingTypingDemoEntry[] = [
  {
    block : '[tool.prose.rules.align-equals]\nenabled   = true\nmax-shift = 8\n\n',
    kind  : 'append',
    slug  : 'align-equals'
  },
  {
    block : '[tool.prose.rules.align-colons]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'align-colons'
  },
  {
    block : '[tool.prose.rules.align-imports]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'align-imports'
  },
  {
    block : '[tool.prose.rules.signature-layout]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'signature-layout'
  },
  {
    block : '[tool.prose.rules.alphabetize]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'alphabetize'
  },
  {
    block : '[tool.prose.rules.no-single-line-docstrings]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'no-single-line-docstrings'
  },
  {
    block : '[tool.prose.rules.docstring-wrap]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'docstring-wrap'
  },
  {
    block : '[tool.prose.rules.blank-lines]\nenabled = true\n\n',
    kind  : 'append',
    slug  : 'blank-lines'
  },
  {
    block : '[tool.prose.rules.collection-layout]\nenabled = true',
    kind  : 'append',
    slug  : 'collection-layout'
  },
  {
    anchor : 'code-line-length      = ',
    from   : '88',
    kind   : 'edit',
    slug   : 'code-line-length',
    tail   : tail({ codeLineLength: 100, docstringLineLength: 76 }),
    to     : '100'
  },
  {
    anchor : 'max-shift = ',
    from   : '8',
    kind   : 'edit',
    slug   : 'max-shift',
    tail   : tail({ codeLineLength: 100, docstringLineLength: 76, maxShift: 6 }),
    to     : '6'
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
