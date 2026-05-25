export type RuleConfigPreset = 'alignment' | 'toggle'

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
    meaning : 'How to handle a group whose widest member exceeds `max-shift`. See [the per-rule knobs](/reference/configuration#per-rule-knobs) for the full semantics',
    type    : '`"split"` | `"drop"` | `"skip"`'
  }
]

const TOGGLE_ROWS: RuleConfigRowSource[] = [ENABLED_ROW]

export const RULE_CONFIG_PRESETS: Record<RuleConfigPreset, RuleConfigRowSource[]> = {
  alignment : ALIGNMENT_ROWS,
  toggle    : TOGGLE_ROWS
}
