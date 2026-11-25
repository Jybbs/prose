import { defineLoader } from 'vitepress'

import * as composition      from './composition.ts'
import { fixtureWatchGlobs } from '../fixtures/walker.ts'
import { fixturesDir }       from '../shared/paths.ts'

const fixturesRoot = fixturesDir(import.meta.url)

declare const data: composition.CompositionData
export { data }

export default defineLoader({
  watch : fixtureWatchGlobs(fixturesRoot),
  load  : () => composition.readCompositionData(composition.compositionDir(fixturesRoot))
})
