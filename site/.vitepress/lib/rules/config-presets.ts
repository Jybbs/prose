export type RuleConfigPreset = 'alignment' | 'toggle'

export interface Row {
  default : string
  key     : string
  meaning : string
  type    : string
}

const ENABLED_ROW: Row = {
  default: 'true',
  key    : 'enabled',
  meaning: 'Toggle the rule on or off',
  type   : 'bool'
}

const ALIGNMENT_ROWS: Row[] = [
  ENABLED_ROW,
  { default: '8',       key: 'max-shift',        meaning: 'Ceiling on per-line padding',                                                                                                                                                                       type: 'positive int' },
  { default: '"split"', key: 'max-shift-policy', meaning: 'How to handle a group whose widest member exceeds <code>max-shift</code>. See <a href="/guide/configuration#per-rule-knobs">the per-rule knobs</a> for the full semantics', type: '<code>"split"</code> | <code>"drop"</code> | <code>"skip"</code>' }
]

const TOGGLE_ROWS: Row[] = [ENABLED_ROW]

export const RULE_CONFIG_PRESETS: Record<RuleConfigPreset, Row[]> = {
  alignment: ALIGNMENT_ROWS,
  toggle   : TOGGLE_ROWS
}
