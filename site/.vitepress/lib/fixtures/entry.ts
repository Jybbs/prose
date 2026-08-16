import { data as fixtures }  from './fixtures.data'
import type { FixtureEntry } from './fixtures.data'
import { lookup }            from '../shared/lookup'

export function fixtureEntry(rule: string, caseName: string): FixtureEntry {
  const cases = lookup(fixtures, rule, 'Fixture rule')
  return lookup(cases, caseName, `Fixture case under "${rule}"`)
}
