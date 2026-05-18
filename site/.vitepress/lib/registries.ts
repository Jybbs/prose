export type RuleCategory = 'auto-fix' | 'lint'

export interface CategoryMeta {
  badge : string
  label : string
}

export const CATEGORY_META: Record<RuleCategory, CategoryMeta> = {
  'auto-fix': { badge: '🪜', label: 'Auto-Fix' },
  'lint'    : { badge: '🧶', label: 'Lint'     }
}

export const PRIMITIVES = {
  'binding-analysis' : 'BindingAnalysis',
  'pipeline'         : 'Pipeline',
  'rule-id'          : 'RuleId',
  'source'           : 'Source',
  'suppression-map'  : 'SuppressionMap'
} as const satisfies Record<string, string>

export type PrimitiveSlug = keyof typeof PRIMITIVES
