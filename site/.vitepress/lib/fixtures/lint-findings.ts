import fs   from 'node:fs'
import path from 'node:path'

import type { DecorationItem } from '@shikijs/types'
import matter                  from 'gray-matter'

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
  const body = matter(fs.readFileSync(snapPath, 'utf8')).content.trim()
  return body ? (JSON.parse(body) as LintFinding[]) : []
}

// Converts findings into shiki decorations that wrap each flagged span
// in a `.lint-flag` element carrying the hover data as `data-*`. Sorted
// by position, since shiki rejects unordered or overlapping ranges.
export function lintDecorations(findings: readonly LintFinding[]): DecorationItem[] {
  return [...findings]
    .sort((a, b) => a.location.row - b.location.row || a.location.column - b.location.column)
    .map(finding => {
      const properties: Record<string, string> = {
        class          : 'lint-flag',
        'data-message' : finding.message,
        'data-rule'    : finding.code
      }
      const suggestion = finding.fix?.edits[0]
      if (suggestion) {
        properties['data-before']    = suggestion.before
        properties['data-suggested'] = suggestion.content
      }
      return {
        end        : { character: finding.end_location.column - 1, line: finding.end_location.row - 1 },
        properties,
        start      : { character: finding.location.column - 1,     line: finding.location.row - 1     }
      }
    })
}
