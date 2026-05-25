export type RuleCategory = 'auto-fix' | 'lint'
export type RuleFamily   = 'alignment' | 'docs' | 'formatting' | 'lint' | 'ordering'

interface CategoryMeta {
  badge : 'A' | 'L'
  label : string
}

interface FamilyMeta {
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

export const FAMILY_ORDER: readonly RuleFamily[] = ['alignment', 'ordering', 'formatting', 'docs', 'lint']

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

export const PRIMITIVE_SLUGS = Object.keys(PRIMITIVES) as readonly PrimitiveSlug[]

export const PUBLIC_PRIMITIVES: readonly PrimitiveSlug[] = ['pipeline', 'rule-id', 'source']

type PrimitiveCoverage = 'exact' | 'subset'

export function assertCoversPrimitives(
  found    : Iterable<string>,
  label    : string,
  coverage : PrimitiveCoverage = 'exact'
): void {
  const knownSet = new Set<string>(Object.keys(PRIMITIVES))
  const foundSet = new Set(found)
  const extra    = [...foundSet.difference(knownSet)]
  const missing  = coverage === 'exact' ? [...knownSet.difference(foundSet)] : []
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${label} out of sync with PRIMITIVES. missing: [${missing.join(', ')}], extra: [${extra.join(', ')}]`
    )
  }
}
