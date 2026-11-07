import path from 'node:path'

import { COMPOSITION_RULE } from '../fixtures/entry'
import * as walker          from '../fixtures/walker'
import { fixturesDirFrom }  from '../shared/paths'

export interface CompositionCase {
  case  : string
  rules : readonly string[]
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
    const rules   = walker.readCaseConfig(path.join(caseDir, walker.INPUT_FILE)).harness?.rules
    if (rules === undefined) {
      throw new Error(`composition: ${caseName}/${walker.CONFIG_FILE} missing [harness].rules`)
    }

    const docs = walker.readFixtureDocs(path.join(caseDir, walker.INPUT_FILE))
    if (docs?.previewable !== true) continue

    cases.push({ case: caseName, rules })
  }
  return cases
}

// Reads the previewable cases and indexes them by rule, returning both for the
// composition page and the rule pages to read.
export function readCompositionData(dir: string): CompositionData {
  const cases = readCompositionCases(dir)
  return { byRule: byRule(cases), cases }
}
