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

const LOWERCASE_IN_TITLE = new Set([
  'a', 'an', 'the',
  'and', 'but', 'or', 'nor', 'so', 'yet',
  'as', 'at', 'by', 'for', 'from', 'in', 'into', 'of', 'on', 'onto', 'to', 'with',
  'among', 'inside', 'over', 'under', 'upon', 'via', 'within'
])

function toTitleCase(slug: string): string {
  const words = slug.split('_')
  return words
    .map((word, i) => {
      const lower = word.toLowerCase()
      if (i !== 0 && i !== words.length - 1 && LOWERCASE_IN_TITLE.has(lower)) {
        return lower
      }
      return lower.charAt(0).toUpperCase() + lower.slice(1)
    })
    .join(' ')
}

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
        title : toTitleCase(caseName)
      })
    }
    return { cases }
  }
})
