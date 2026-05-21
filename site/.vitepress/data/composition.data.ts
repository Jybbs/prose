import fs   from 'node:fs/promises'
import path from 'node:path'

import { parse }        from 'smol-toml'
import { defineLoader } from 'vitepress'

import { repoRoot }     from '../lib/shared/paths'
import { toTitleCase }  from '../lib/shared/title-case'

export interface CompositionCase {
  case  : string
  rules : readonly string[]
  title : string
}

export interface CompositionData {
  cases : readonly CompositionCase[]
}

interface CompositionToml {
  harness : { rules: readonly string[] }
}

const compositionDir = path.join(repoRoot(import.meta.url), 'tests/fixtures/composition')

declare const data: CompositionData
export { data }

export default defineLoader({
  watch: [`${compositionDir}/*.config.toml`],
  async load(): Promise<CompositionData> {
    const files = (await fs.readdir(compositionDir)).filter(f => f.endsWith('.config.toml')).sort()
    const cases = await Promise.all(files.map(async file => {
      const caseName = path.basename(file, '.config.toml')
      const parsed   = parse(await fs.readFile(path.join(compositionDir, file), 'utf8')) as unknown as CompositionToml
      return {
        case  : caseName,
        rules : parsed.harness.rules,
        title : toTitleCase(caseName)
      }
    }))
    return { cases }
  }
})
