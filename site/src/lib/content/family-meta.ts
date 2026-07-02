import { familyBadges }      from './rules'
import type { RuleCategory } from '../shared/registries'
import { titleCase }         from '../shared/title-case'

export interface FamilyMeta {
  badge : string
  label : string
}

export const CATEGORY_META: Record<RuleCategory, FamilyMeta> = {
  'auto-fix' : { badge: 'A', label: 'Auto-Fix' },
  lint       : { badge: 'L', label: 'Lint'     }
}

// The glossary spans two families beyond the rule set, whose badges have no
// index page to read from, so they carry their meta here.
const EXTRA_FAMILY_META: Record<string, FamilyMeta> = {
  cli    : { badge: '🪄', label: 'CLI'    },
  engine : { badge: '🦉', label: 'Engine' }
}

let cached: Promise<ReadonlyMap<string, FamilyMeta>> | null = null

// One badge-and-label per family-axis entry, the rule families derived from
// the discovery and the glossary-only families curated above.
export function familyMeta(): Promise<ReadonlyMap<string, FamilyMeta>> {
  return (cached ??= deriveMeta())
}

async function deriveMeta(): Promise<ReadonlyMap<string, FamilyMeta>> {
  const meta = new Map(Object.entries(EXTRA_FAMILY_META))
  for (const [family, badge] of await familyBadges()) {
    meta.set(family, { badge, label: titleCase(family) })
  }
  return meta
}
