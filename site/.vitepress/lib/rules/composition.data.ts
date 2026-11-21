import { defineLoader } from 'vitepress'

import * as composition      from './composition.ts'
import { fixtureWatchGlobs } from '../fixtures/walker.ts'
import { crateDir }          from '../shared/paths.ts'

const crate = crateDir(import.meta.url)

declare const data: composition.CompositionData
export { data }

export default defineLoader({
  watch : fixtureWatchGlobs(crate),
  load  : () => composition.readCompositionData(composition.compositionDir(crate))
})
