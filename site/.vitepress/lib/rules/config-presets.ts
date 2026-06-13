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
    default : '16',
    key     : 'max-shift',
    meaning : 'Width-spread budget for an alignment run. A positive `N` caps the spread, '
            + '`0` forbids any shift, and `false` folds a contiguous run into one column',
    type    : 'positive int | `0` | `false`'
  }
]

const TOGGLE_ROWS: RuleConfigRowSource[] = [ENABLED_ROW]

export const RULE_CONFIG_PRESETS: Record<RuleConfigPreset, RuleConfigRowSource[]> = {
  alignment : ALIGNMENT_ROWS,
  toggle    : TOGGLE_ROWS
}
