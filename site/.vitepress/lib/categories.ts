export type RuleCategory = 'auto-fix' | 'lint'

export interface CategoryMeta {
  badge : string
  label : string
}

export const CATEGORY_META: Record<RuleCategory, CategoryMeta> = {
  'auto-fix': { badge: '🪜', label: 'Auto-Fix' },
  'lint'    : { badge: '🧶', label: 'Lint'     }
}
