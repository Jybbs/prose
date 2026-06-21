import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

export const LINT_FINDINGS_FILE = 'lint_findings.snap'

interface FindingEdit {
  before  : string
  content : string
}

interface FindingLocation {
  column : number
  row    : number
}

export interface LintFinding {
  code          : string
  end_location  : FindingLocation
  fix          ?: { applicability: string, edits: FindingEdit[] }
  location      : FindingLocation
  message       : string
}

// Reads a composition case's lint findings from its `lint_findings.snap`
// sidecar, the harness-emitted JSON records keyed to the formatted
// output. Returns `[]` for cases that carry no sidecar.
export function readLintFindings(inputPath: string): LintFinding[] {
  const snapPath = path.join(path.dirname(inputPath), LINT_FINDINGS_FILE)
  if (!fs.existsSync(snapPath)) return []
  const body = matter.read(snapPath).content.trim()
  return body ? (JSON.parse(body) as LintFinding[]) : []
}
