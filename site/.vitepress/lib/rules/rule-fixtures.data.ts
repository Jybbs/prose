import { defineLoader } from 'vitepress'

import { readRuleFixtures, type RuleFixturesData } from './rule-fixtures'
import * as walker                                 from '../fixtures/walker'
import { crateDir }                                from '../shared/paths'

const crate = crateDir(import.meta.url)

declare const data: RuleFixturesData
export { data }

export default defineLoader({
  watch : walker.fixtureWatchGlobs(crate),
  load  : () => readRuleFixtures(crate)
})
