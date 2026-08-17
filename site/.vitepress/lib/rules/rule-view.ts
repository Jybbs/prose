import { data as composition }  from './composition.data'
import { data as ruleFixtures } from './rule-fixtures.data'
import type { RuleFixtureSet }  from './rule-fixtures.data'
import type { RenderedRule }    from './rules.data'
import { lookup }               from '../shared/lookup'
import { ruleSlug }             from '../shared/rule-slug'

// One rule's place in a composition case, the index counting from one in
// pipeline order and both other fields resolving to null for an unregistered
// slug.
export interface RuleSegment {
  family : string | null
  index  : number
  rule   : RenderedRule | null
  slug   : string
}

// The previewable composition cases a rule takes part in, empty where none do.
export function casesForRule(fixtureRule: string): readonly string[] {
  return composition.byRule[ruleSlug(fixtureRule)] ?? []
}

// The canonical case and further examples registered for a rule.
export function fixturesForRule(fixtureRule: string): RuleFixtureSet {
  return lookup(ruleFixtures, fixtureRule, 'Rule')
}
