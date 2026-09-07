import { defineLoader } from 'vitepress'

import { readRuleFixtures, type RuleFixturesData } from './rule-fixtures'
import { fixtureWatchGlobs }                       from '../fixtures/walker'
import { crateDir }                                from '../shared/paths'

const crate = crateDir(import.meta.url)

declare const data: RuleFixturesData
export { data }

export default defineLoader({
  watch : fixtureWatchGlobs(crate),
  load  : () => readRuleFixtures(crate)
})
