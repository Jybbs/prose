type ScopeKey = 'block' | 'dict' | 'file' | 'line'

type Intent = 'format' | 'lint' | 'order'

interface Decision {
  detail        : string
  directive     : string
  id            : string
  intent        : Intent
  scope         : ScopeKey
  situation     : string
  situationLead : string
}

export const SCOPE_META: Record<ScopeKey, { label: string; pip: string; rank: number }> = {
  block : { label : 'Block',         pip : 'B', rank : 2 },
  dict  : { label : 'Dict literal',  pip : 'D', rank : 4 },
  file  : { label : 'File',          pip : 'F', rank : 1 },
  line  : { label : 'Line',          pip : 'L', rank : 3 }
}

export const SCOPE_ORDER: ScopeKey[] = ['file', 'block', 'line', 'dict']

export const DECISIONS: Decision[] = [
  {
    detail        : 'Useful for a generated file, an embedded vendored module, or any source where '
                  + 'Prose would fight the existing layout.',
    directive     : '# prose: off',
    id            : 'file-off',
    intent        : 'format',
    scope         : 'file',
    situation     : 'A whole file is hand-tuned and the formatter has nothing to add.',
    situationLead : 'Whole file hand-tuned'
  },
  {
    detail        : 'The opening line is the marker, and formatting resumes after the closing '
                  + 'line.',
    directive     : '# fmt: off … # fmt: on',
    id            : 'block-fmt',
    intent        : 'format',
    scope         : 'block',
    situation     : 'A block of code is a hand-tuned data table, ASCII diagram, or otherwise '
                  + 'carries layout that the formatter would smooth over.',
    situationLead : 'Hand-tuned block'
  },
  {
    detail        : 'The right shape when only one statement carries the exception, because '
                  + 'wrapping a single line in fmt off-on reads heavier than the exception '
                  + 'warrants.',
    directive     : '# fmt: skip',
    id            : 'line-skip',
    intent        : 'format',
    scope         : 'line',
    situation     : 'A single line wants the formatter to leave it alone.',
    situationLead : 'Single line, no format'
  },
  {
    detail        : 'Use [<rule>] for one rule, [a, b] for several. The other auto-fix rules stay '
                  + 'free to run on the line.',
    directive     : '# prose: skip[<rule>]',
    id            : 'line-skip-rules',
    intent        : 'format',
    scope         : 'line',
    situation     : 'A single line wants specific rewrite rules to leave it alone, with the others '
                  + 'free to fire.',
    situationLead : 'Single line, narrow rewrite skip'
  },
  {
    detail        : 'Use [<rule>] for one rule, [a, b] for several, or bare ignore for every lint '
                  + 'on the line.',
    directive     : '# prose: ignore[<rule>]',
    id            : 'line-ignore',
    intent        : 'lint',
    scope         : 'line',
    situation     : 'A single line wants to silence a lint diagnostic.',
    situationLead : 'Single line, silence lint'
  },
  {
    detail        : 'Tells alphabetize to leave the entries alone. The directive scopes only to '
                  + 'that one dict.',
    directive     : '# prose: keep',
    id            : 'dict-keep',
    intent        : 'order',
    scope         : 'dict',
    situation     : 'A dict literal’s entry order carries meaning a future reader needs preserved, '
                  + 'like a pipeline-stage sequence or a column layout.',
    situationLead : 'Dict order matters'
  }
]
