import fs   from 'node:fs/promises'
import path from 'node:path'

import { parse }        from 'smol-toml'
import { defineLoader } from 'vitepress'

import { subdirNames } from '../lib/fixtures/walker'
import { crateDir }    from '../lib/shared/paths'
import { toTitleCase } from '../lib/shared/title-case'

interface CompositionCase {
  case  : string
  rules : readonly string[]
  title : string
}

interface CompositionData {
  cases : readonly CompositionCase[]
}

const compositionDir = path.join(crateDir(import.meta.url), 'tests/fixtures/composition')

declare const data: CompositionData
export { data }

export default defineLoader({
  watch: [`${compositionDir}/*/config.toml`],
  async load(): Promise<CompositionData> {
    const caseDirs = subdirNames(compositionDir)
    const cases = await Promise.all(caseDirs.map(async caseName => {
      type Parsed  = { harness?: { rules?: readonly string[] } }
      const config = path.join(compositionDir, caseName, 'config.toml')
      const parsed = parse(await fs.readFile(config, 'utf8')) as Parsed
      const rules  = parsed.harness?.rules
      if (rules === undefined) {
        throw new Error(`composition.data: ${caseName}/config.toml missing [harness].rules`)
      }
      return {
        case  : caseName,
        rules,
        title : toTitleCase(caseName)
      }
    }))
    return { cases }
  }
})
