export const RULE_CHIP_SELECTOR = 'a[data-rule-pop]'

interface RuleChipSource {
  badge   : string
  caption : string
  family  : string
}

// The hover-contract dataset every rule-chip producer emits, the bare
// `data-rule-pop` marker keying the overlay's selector.
export function ruleChipAttrs(rule: RuleChipSource): Record<string, string> {
  return { ...ruleHoverAttrs(rule), 'data-rule-pop': '' }
}

export function ruleHoverAttrs(rule: RuleChipSource): Record<string, string> {
  return {
    'data-badge'   : rule.badge,
    'data-caption' : rule.caption,
    'data-family'  : rule.family
  }
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('rule-chip', () => {
    test('selects the bare hover marker', () => {
      expect(RULE_CHIP_SELECTOR).toBe('a[data-rule-pop]')
    })

    test('emits the hover-contract dataset with an empty marker', () => {
      expect(ruleChipAttrs({ badge: '📐', caption: 'aligns `=`', family: 'alignment' })).toMatchInlineSnapshot(`
        {
          "data-badge": "📐",
          "data-caption": "aligns \`=\`",
          "data-family": "alignment",
          "data-rule-pop": "",
        }
      `)
    })
  })
}
