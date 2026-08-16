import { data as composition }  from './composition.data'
import { data as ruleFixtures } from './rule-fixtures.data'
import type { RuleFixtureSet }  from './rule-fixtures.data'
import { lookup }               from '../shared/lookup'
import { ruleSlug }             from '../shared/rule-slug'

// The previewable composition cases a rule takes part in, empty where none do.
export function casesForRule(fixtureRule: string): readonly string[] {
  return composition.byRule[ruleSlug(fixtureRule)] ?? []
}

// The canonical case and further examples registered for a rule.
export function fixturesForRule(fixtureRule: string): RuleFixtureSet {
  return lookup(ruleFixtures, fixtureRule, 'Rule')
}
