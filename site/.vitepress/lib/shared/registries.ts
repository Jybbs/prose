export type RuleCategory   = 'auto-fix' | 'lint'
export type RuleFamily     = 'alignment' | 'docs' | 'formatting' | 'layout' | 'lint' | 'ordering'
export type GlossaryFamily = RuleFamily | 'cli' | 'engine'

interface CategoryMeta {
  badge : 'A' | 'L'
  label : string
}

interface FamilyMeta {
  badge  : string
  label  : string
  warmth : 'cool' | 'warm'
}

export const CATEGORY_META: Record<RuleCategory, CategoryMeta> = {
  'auto-fix' : { badge: 'A', label: 'Auto-Fix' },
  'lint'     : { badge: 'L', label: 'Lint'     }
}

export const FAMILY_META: Record<RuleFamily, FamilyMeta> = {
  alignment  : { badge: '🪜', label: 'Alignment',  warmth: 'warm' },
  docs       : { badge: '📰', label: 'Docs',       warmth: 'cool' },
  formatting : { badge: '🪶', label: 'Formatting', warmth: 'warm' },
  layout     : { badge: '🧺', label: 'Layout',     warmth: 'cool' },
  lint       : { badge: '🧶', label: 'Lint',       warmth: 'warm' },
  ordering   : { badge: '🪉', label: 'Ordering',   warmth: 'cool' }
}

export const GLOSSARY_FAMILY_META: Record<GlossaryFamily, Pick<FamilyMeta, 'badge' | 'label'>> = {
  ...FAMILY_META,
  cli    : { badge: '🪄', label: 'CLI'    },
  engine : { badge: '🦉', label: 'Engine' }
}

export const FAMILY_ORDER: readonly RuleFamily[] = [
  'alignment', 'docs', 'formatting', 'layout', 'lint', 'ordering'
]

// Reading order, not alphabetical, because the list order drives the nav.
export const SECTIONS = [
  { label: 'Usage',        slug: 'usage'        },
  { label: 'Reference',    slug: 'reference'    },
  { label: 'Integrations', slug: 'integrations' },
  { label: 'Rules',        slug: 'rules'        },
  { label: 'Primitives',   slug: 'primitives'   },
  { label: 'Sandbox',      slug: 'sandbox'      }
] as const

export type SectionSlug = (typeof SECTIONS)[number]['slug']

export function categoryOf(family: RuleFamily): RuleCategory {
  return family === 'lint' ? 'lint' : 'auto-fix'
}

export type PrimitiveSlug =
  | 'aligner' | 'binding-analysis' | 'cache' | 'colon-targets' | 'docstring' | 'edit'
  | 'orderer' | 'pipeline' | 'rule-id' | 'source' | 'suppression-map' | 'walker'

export const PRIMITIVE_LAYERS      = ['analysis', 'base', 'orchestration'] as const
export const PRIMITIVE_STABILITIES = ['internal', 'public'] as const

export type PrimitiveLayer     = (typeof PRIMITIVE_LAYERS)[number]
export type PrimitiveStability = (typeof PRIMITIVE_STABILITIES)[number]

export const PRIMITIVE_LAYER_NUMERALS: Record<PrimitiveLayer, string> = {
  analysis      : 'III',
  base          : 'I',
  orchestration : 'II'
}
