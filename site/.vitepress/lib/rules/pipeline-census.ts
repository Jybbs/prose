import type { RuleFamily } from '../shared/registries'

export interface CensusEntry {
  count  : number
  family : RuleFamily
}

// Counts each family's passes among the passes up to and including `through`,
// or among every pass where `through` is null, listing the families in the
// order of their first pass so a family the count has not reached reads zero.
export function passCensus(
  families : readonly (RuleFamily | null)[],
  through  : number | null = null
): CensusEntry[] {
  const counts = new Map<RuleFamily, number>()
  families.forEach((family, at) => {
    if (family) counts.set(family, (counts.get(family) ?? 0) + Number(through === null || at <= through))
  })
  return [...counts].map(([family, count]) => ({ count, family }))
}
