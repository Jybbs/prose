import type { CompositionCase }           from './composition'
import { data as composition }            from './composition.data'
import type { RuleFixtureSet }            from './rule-fixtures'
import { data as ruleFixtures }           from './rule-fixtures.data'
import type { RenderedRule }              from './rules.data'
import { COMPOSITION_RULE, embedsInline } from '../fixtures/entry'
import { lookup }                         from '../shared/lookup'
import { ruleSlug }                       from '../shared/rule-slug'

// One rule's place in a composition case, the index counting from one in
// pipeline order and both other fields resolving to null for an unregistered
// slug.
export interface RuleSegment {
  family : string | null
  index  : number
  rule   : RenderedRule | null
  slug   : string
}

// Lists the previewable composition cases a rule takes part in, empty where the
// rule takes part in none.
export function casesForRule(fixtureRule: string): readonly string[] {
  return composition.byRule[ruleSlug(fixtureRule)] ?? []
}

// Looks up the canonical case and further examples registered for a rule.
export function fixturesForRule(fixtureRule: string): RuleFixtureSet {
  return lookup(ruleFixtures, fixtureRule, 'Rule')
}

// Lists the previewable composition cases a page shows as cards, narrowed to
// the ones a rule takes part in where the page names one, leaving out each case
// the page embeds inline.
export function pageCompositionCases(
  frontmatter  : Record<string, unknown>,
  fixtureRule ?: string
): readonly CompositionCase[] {
  const participating = fixtureRule === undefined ? null : new Set(casesForRule(fixtureRule))
  return composition.cases.filter(entry =>
    (participating === null || participating.has(entry.case)) &&
    !embedsInline(frontmatter, COMPOSITION_RULE, entry.case))
}

// Lists the previewable examples a rule's page shows, leaving out each one the
// page embeds inline.
export function pageExamples(
  frontmatter : Record<string, unknown>,
  fixtureRule : string
): RuleFixtureSet['examples'] {
  return fixturesForRule(fixtureRule).examples
    .filter(example => !embedsInline(frontmatter, fixtureRule, example.case))
}
