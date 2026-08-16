import fs   from 'node:fs'
import path from 'node:path'

import { stringify } from 'smol-toml'

import * as walker     from '../fixtures/walker'
import { parseToml }   from '../shared/toml'
import { toTitleCase } from '../shared/title-case'

type CaseConfig = Record<string, unknown> & { harness?: { rules?: readonly string[] } }

export interface CompositionCase {
  case       : string
  configToml : string
  rules      : readonly string[]
  source     : string
  title      : string
}

// Inverts the per-case rule list, each rule's cases holding the order the
// composition page lists them in.
export function byRule(cases: readonly CompositionCase[]): Record<string, readonly string[]> {
  const index: Record<string, string[]> = {}
  for (const entry of cases) {
    for (const slug of entry.rules) (index[slug] ??= []).push(entry.case)
  }
  return index
}

// Reads the cases a `meta.toml` marks previewable.
export function readCompositionCases(compositionDir: string): CompositionCase[] {
  const cases: CompositionCase[] = []
  for (const caseName of walker.subdirNames(compositionDir)) {
    const caseDir   = path.join(compositionDir, caseName)
    const inputPath = path.join(caseDir, 'input.py')
    const docs      = walker.readFixtureDocs(inputPath)
    if (docs?.previewable !== true) continue

    const config = parseToml(path.join(caseDir, 'config.toml')) as CaseConfig
    const rules  = config.harness?.rules
    if (rules === undefined) {
      throw new Error(`composition: ${caseName}/config.toml missing [harness].rules`)
    }
    cases.push({
      case       : caseName,
      configToml : seedToml(config),
      rules,
      source     : fs.readFileSync(inputPath, 'utf8'),
      title      : walker.fixtureTitle(docs) ?? toTitleCase(caseName)
    })
  }
  return cases
}

// Everything outside `[harness]` is the prose config the case formats under.
export function seedToml(config: Record<string, unknown>): string {
  const overrides = { ...config }
  delete overrides.harness
  const text = stringify(overrides)
  return text.trim() ? text : ''
}
