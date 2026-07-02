// The hand-curated taxonomy unions that the content schemas and the integrity
// check read. Runtime classification flows from frontmatter and the directory
// tree, so this module carries only the closed vocabularies a Zod enum and the
// cross-record pass validate against.

const FAMILY_ORDER = ['alignment', 'docs', 'formatting', 'layout', 'lint', 'ordering'] as const
export type RuleFamily = (typeof FAMILY_ORDER)[number]

export const isFamily = (name: string): name is RuleFamily =>
  (FAMILY_ORDER as readonly string[]).includes(name)

export const FAMILY_WARMTHS = ['cool', 'warm'] as const

export const GLOSSARY_FAMILIES = [...FAMILY_ORDER, 'cli', 'engine'] as const

export const PRIMITIVE_LAYERS = ['analysis', 'base', 'orchestration'] as const

export const PRIMITIVE_STABILITIES = ['internal', 'public'] as const
