import { familyBadges }      from './rules'
import { lazy }              from '../../shared/lazy'
import type { RuleCategory } from '../../shared/registries'
import { titleCase }         from '../../shared/title-case'

interface FamilyMeta {
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

// One badge-and-label lookup per family-axis entry, the rule families derived
// from the discovery, the glossary-only families curated above, and an unknown
// family falling back to a bare label.
export const familyMeta = lazy(deriveMeta)

async function deriveMeta(): Promise<(family: string) => FamilyMeta> {
  const meta = new Map(Object.entries(EXTRA_FAMILY_META))
  for (const [family, badge] of await familyBadges()) {
    meta.set(family, { badge, label: titleCase(family) })
  }
  return family => meta.get(family) ?? { badge: '', label: family }
}
