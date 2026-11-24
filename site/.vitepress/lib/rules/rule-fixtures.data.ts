import { defineLoader } from 'vitepress'

import { readRuleFixtures, type RuleFixturesData } from './rule-fixtures.ts'
import { fixtureWatchGlobs }                       from '../fixtures/walker.ts'
import { fixturesDir }                             from '../shared/paths.ts'

const fixturesRoot = fixturesDir(import.meta.url)

declare const data: RuleFixturesData
export { data }

export default defineLoader({
  watch : fixtureWatchGlobs(fixturesRoot),
  load  : () => readRuleFixtures(fixturesRoot)
})
