export type RuleCategory   = 'auto-fix' | 'lint'
export type RuleFamily     = 'alignment' | 'docs' | 'formatting' | 'lint' | 'ordering'
export type GlossaryFamily = RuleFamily | 'engine'

interface CategoryMeta {
  badge : 'A' | 'L'
  label : string
}

interface FamilyMeta {
  badge  : string
  color  : string
  label  : string
  warmth : 'cool' | 'warm'
}

export const CATEGORY_META: Record<RuleCategory, CategoryMeta> = {
  'auto-fix' : { badge: 'A', label: 'Auto-Fix' },
  'lint'     : { badge: 'L', label: 'Lint'     }
}

export const FAMILY_META: Record<RuleFamily, FamilyMeta> = {
  alignment  : { badge: '🪜', color: '#e8c840', label: 'Alignment',  warmth: 'warm' },
  docs       : { badge: '📰', color: '#8cc5a3', label: 'Docs',       warmth: 'cool' },
  formatting : { badge: '🪶', color: '#c08597', label: 'Formatting', warmth: 'warm' },
  lint       : { badge: '🧶', color: '#e8876f', label: 'Lint',       warmth: 'warm' },
  ordering   : { badge: '🪉', color: '#7db3e0', label: 'Ordering',   warmth: 'cool' }
}

export const GLOSSARY_FAMILY_META: Record<GlossaryFamily, FamilyMeta> = {
  ...FAMILY_META,
  engine: { badge: '🦉', color: '#8a80cb', label: 'Engine', warmth: 'cool' }
}

export const FAMILY_ORDER: readonly RuleFamily[] = [
  'alignment', 'ordering', 'formatting', 'docs', 'lint'
]

export type PrimitiveSlug =
  | 'aligner' | 'binding-analysis' | 'cache' | 'colon-targets' | 'docstring' | 'edit'
  | 'orderer' | 'pipeline' | 'rule-id' | 'source' | 'suppression-map' | 'walker'
