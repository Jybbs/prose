export type RuleCategory = 'auto-fix' | 'lint'
export type RuleFamily   = 'alignment' | 'ordering' | 'formatting' | 'docs' | 'lint'

export interface CategoryMeta {
  badge : 'A' | 'L'
  label : string
}

export interface FamilyMeta {
  badge : string
  label : string
}

export const CATEGORY_META: Record<RuleCategory, CategoryMeta> = {
  'auto-fix' : { badge: 'A', label: 'Auto-Fix' },
  'lint'     : { badge: 'L', label: 'Lint'     }
}

export const FAMILY_META: Record<RuleFamily, FamilyMeta> = {
  alignment  : { badge: '🪜', label: 'Alignment'  },
  docs       : { badge: '📰', label: 'Docs'       },
  formatting : { badge: '🪶', label: 'Formatting' },
  lint       : { badge: '🧶', label: 'Lint'       },
  ordering   : { badge: '🪉', label: 'Ordering'   }
}

export const PRIMITIVES = {
  'aligner'          : 'Aligner',
  'binding-analysis' : 'BindingAnalysis',
  'colon-targets'    : 'ColonTargets',
  'docstring'        : 'Docstring',
  'edit'             : 'Edit',
  'orderer'          : 'Orderer',
  'pipeline'         : 'Pipeline',
  'rule-id'          : 'RuleId',
  'source'           : 'Source',
  'suppression-map'  : 'SuppressionMap',
  'walker'           : 'Walker'
} as const satisfies Record<string, string>

export type PrimitiveSlug = keyof typeof PRIMITIVES

export const PUBLIC_PRIMITIVES: readonly PrimitiveSlug[] = ['pipeline', 'rule-id', 'source']

export type PrimitiveCoverage = 'exact' | 'subset'

export function assertCoversPrimitives(
  found    : Iterable<string>,
  label    : string,
  coverage : PrimitiveCoverage = 'exact'
): void {
  const knownSet = new Set(Object.keys(PRIMITIVES))
  const foundSet = new Set(found)
  const extra    = [...foundSet].filter(s => !knownSet.has(s))
  const missing  = coverage === 'exact'
    ? [...knownSet].filter(s => !foundSet.has(s))
    : []
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${label} out of sync with PRIMITIVES. missing: [${missing.join(', ')}], extra: [${extra.join(', ')}]`
    )
  }
}
