import fs   from 'node:fs'
import path from 'node:path'

import { COMPOSITION_RULE } from '../fixtures/entry'
import * as walker          from '../fixtures/walker'
import { inlineCode }       from '../shared/inline-code'
import { fixturesDirFrom }  from '../shared/paths'
import { toTitleCase }      from '../shared/title-case'
import { parseToml }        from '../shared/toml'
import { tomlText }         from '../shared/toml-text'

type CaseConfig = Record<string, unknown> & { harness?: { rules?: readonly string[] } }

export interface CompositionCase {
  case       : string
  configToml : string
  rules      : readonly string[]
  source     : string
  titleHtml  : string
}

export interface CompositionData {
  byRule : Record<string, readonly string[]>
  cases  : readonly CompositionCase[]
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

// Resolves the composition directory inside one crate's fixture tree.
export function compositionDir(crate: string): string {
  return path.join(fixturesDirFrom(crate), COMPOSITION_RULE)
}

// Validates every case's harness rules and reads the ones a `meta.toml` marks
// previewable.
export function readCompositionCases(dir: string): CompositionCase[] {
  const cases: CompositionCase[] = []
  for (const caseName of walker.subdirNames(dir)) {
    const caseDir = path.join(dir, caseName)
    const config  = parseToml(path.join(caseDir, walker.CONFIG_FILE)) as CaseConfig
    const rules   = config.harness?.rules
    if (rules === undefined) {
      throw new Error(`composition: ${caseName}/${walker.CONFIG_FILE} missing [harness].rules`)
    }

    const inputPath = path.join(caseDir, walker.INPUT_FILE)
    const docs      = walker.readFixtureDocs(inputPath)
    if (docs?.previewable !== true) continue

    cases.push({
      case       : caseName,
      configToml : seedToml(config),
      rules,
      source     : fs.readFileSync(inputPath, 'utf8'),
      titleHtml  : inlineCode(walker.fixtureTitle(docs) ?? toTitleCase(caseName))
    })
  }
  return cases
}

// Reads the previewable cases and indexes them by rule, returning both for the
// composition page and the rule pages to read.
export function readCompositionData(dir: string): CompositionData {
  const cases = readCompositionCases(dir)
  return { byRule: byRule(cases), cases }
}

// Renders everything outside `[harness]`, which is the prose config the case
// formats under.
export function seedToml(config: Record<string, unknown>): string {
  const { harness: _, ...overrides } = config
  return tomlText(overrides)
}
