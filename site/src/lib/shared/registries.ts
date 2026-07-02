// Runtime classification flows from frontmatter and the directory tree, so this
// module carries only the closed vocabularies the Zod enums validate against.

export const FAMILY_ORDER = ['alignment', 'docs', 'formatting', 'layout', 'lint', 'ordering'] as const
export type RuleFamily = (typeof FAMILY_ORDER)[number]

export const isFamily = (name: string): name is RuleFamily =>
  (FAMILY_ORDER as readonly string[]).includes(name)

export type RuleCategory = 'auto-fix' | 'lint'

// Lint coincides with its domain, so the category collapses off the family.
export const categoryOf = (family: string): RuleCategory =>
  family === 'lint' ? 'lint' : 'auto-fix'

export const FAMILY_WARMTHS = ['cool', 'warm'] as const

export const GLOSSARY_FAMILIES = [...FAMILY_ORDER, 'cli', 'engine'] as const
export type GlossaryFamily = (typeof GLOSSARY_FAMILIES)[number]

export const PRIMITIVE_LAYERS = ['analysis', 'base', 'orchestration'] as const

export const PRIMITIVE_STABILITIES = ['internal', 'public'] as const
export type PrimitiveStability = (typeof PRIMITIVE_STABILITIES)[number]
