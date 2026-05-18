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

export function assertCoversPrimitives(found: Iterable<string>, label: string): void {
  const knownSet = new Set(Object.keys(PRIMITIVES))
  const foundSet = new Set(found)
  const missing  = [...knownSet].filter(s => !foundSet.has(s))
  const extra    = [...foundSet].filter(s => !knownSet.has(s))
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${label} out of sync with PRIMITIVES. missing: [${missing.join(', ')}], extra: [${extra.join(', ')}]`
    )
  }
}
