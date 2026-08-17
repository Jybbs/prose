import path from 'node:path'

import { defineLoader } from 'vitepress'

import * as composition              from './composition'
import { fixtureWatchGlobs }         from '../fixtures/walker'
import { crateDir, fixturesDirFrom } from '../shared/paths'

interface CompositionData {
  byRule : Record<string, readonly string[]>
  cases  : readonly composition.CompositionCase[]
}

const crate          = crateDir(import.meta.url)
const compositionDir = path.join(fixturesDirFrom(crate), 'composition')

declare const data: CompositionData
export { data }

export default defineLoader({
  watch: fixtureWatchGlobs(crate),
  load(): CompositionData {
    const cases = composition.readCompositionCases(compositionDir)
    return { byRule: composition.byRule(cases), cases }
  }
})
