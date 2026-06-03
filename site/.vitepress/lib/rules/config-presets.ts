export type RuleConfigPreset = 'align-imports' | 'alignment' | 'toggle'

interface RuleConfigRowSource {
  default : string
  key     : string
  meaning : string
  type    : string
}

const ENABLED_ROW: RuleConfigRowSource = {
  default : 'true',
  key     : 'enabled',
  meaning : 'Toggle the rule on or off',
  type    : 'bool'
}

const ALIGNMENT_ROWS: RuleConfigRowSource[] = [
  ENABLED_ROW,
  {
    default : '8',
    key     : 'max-shift',
    meaning : 'Ceiling on per-line padding',
    type    : 'positive int'
  },
  {
    default : '"split"',
    key     : 'max-shift-policy',
    meaning : 'How to handle a group whose widest member exceeds `max-shift`. See [the '
            + 'per-rule knobs](/reference/configuration#per-rule-knobs) for the full '
            + 'semantics',
    type    : '`"split"` | `"drop"`'
  }
]

// `align-imports` seeds a wider `max-shift` default than the
// operator-alignment rules, so its preset overrides that one cell.
const ALIGN_IMPORTS_ROWS: RuleConfigRowSource[] = ALIGNMENT_ROWS.map(row =>
  row.key === 'max-shift' ? { ...row, default: '16' } : row
)

const TOGGLE_ROWS: RuleConfigRowSource[] = [ENABLED_ROW]

export const RULE_CONFIG_PRESETS: Record<RuleConfigPreset, RuleConfigRowSource[]> = {
  'align-imports' : ALIGN_IMPORTS_ROWS,
  alignment       : ALIGNMENT_ROWS,
  toggle          : TOGGLE_ROWS
}
