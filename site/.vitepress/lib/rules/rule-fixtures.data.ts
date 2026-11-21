import { defineLoader } from 'vitepress'

import { readRuleFixtures, type RuleFixturesData } from './rule-fixtures.ts'
import { fixtureWatchGlobs }                       from '../fixtures/walker.ts'
import { crateDir }                                from '../shared/paths.ts'

const crate = crateDir(import.meta.url)

declare const data: RuleFixturesData
export { data }

export default defineLoader({
  watch : fixtureWatchGlobs(crate),
  load  : () => readRuleFixtures(crate)
})
