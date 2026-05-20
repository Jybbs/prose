import fs   from 'node:fs'
import path from 'node:path'

import { parse }        from 'smol-toml'
import { defineLoader } from 'vitepress'

import { repoRoot } from '../lib/shared/paths'

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
    const cases: CompositionCase[] = []
    for (const file of fs.readdirSync(compositionDir).sort()) {
      if (!file.endsWith('.config.toml')) continue
      const caseName = path.basename(file, '.config.toml')
      const parsed   = parse(fs.readFileSync(path.join(compositionDir, file), 'utf8')) as unknown as CompositionToml
      cases.push({
        case  : caseName,
        rules : parsed.harness.rules,
        title : caseName.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
      })
    }
    return { cases }
  }
})
