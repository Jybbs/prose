import { defineLoader } from 'vitepress'

import * as composition      from './composition'
import { fixtureWatchGlobs } from '../fixtures/walker'
import { crateDir }          from '../shared/paths'

const crate = crateDir(import.meta.url)

declare const data: composition.CompositionData
export { data }

export default defineLoader({
  watch : fixtureWatchGlobs(crate),
  load  : () => composition.readCompositionData(composition.compositionDir(crate))
})
