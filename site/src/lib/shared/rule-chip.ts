export const RULE_CHIP_SELECTOR = 'a[data-rule-pop]'

interface RuleChipSource {
  badge   : string
  caption : string
  family  : string
}

// The hover-contract dataset every rule-chip producer emits, the bare
// `data-rule-pop` marker keying the overlay's selector.
export function ruleChipAttrs(rule: RuleChipSource): Record<string, string> {
  return {
    'data-badge'    : rule.badge,
    'data-caption'  : rule.caption,
    'data-family'   : rule.family,
    'data-rule-pop' : ''
  }
}
