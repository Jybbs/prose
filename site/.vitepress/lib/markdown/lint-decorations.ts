import type { DecorationItem, ShikiTransformer } from '@shikijs/types'

import type { LintFinding } from '../fixtures/lint-findings'

const FLAG_CLASS  = 'lint-flag underline-draw'
const LINE_CLASS  = 'lint-flag lint-flag-line'
const META_PREFIX = 'lint='

// Whether the span opens at the first column and closes at the end of its
// last line. Columns count UTF scalar values, matching what the emitting
// `LineColumn` reports.
function coversWholeLines(finding: LintFinding, lines: readonly string[]): boolean {
  const last = lines[finding.end_location.row - 1]
  return finding.location.column === 1
    && last !== undefined
    && finding.end_location.column === [...last].length + 1
}

// Converts findings into shiki decorations carrying the hover data as
// `data-*`. A token span lands on a `.lint-flag` element of its own,
// whereas a span covering whole rows merges onto the line element and
// takes the row treatment. Sorted by position, since shiki rejects
// unordered ranges.
export function lintDecorations(findings: readonly LintFinding[], code: string): DecorationItem[] {
  const lines = code.split('\n')
  return findings
    .toSorted((a, b) => a.location.row - b.location.row || a.location.column - b.location.column)
    .map(finding => {
      const edit = finding.fix?.edits[0]
      const properties: Record<string, string> = {
        class          : coversWholeLines(finding, lines) ? LINE_CLASS : FLAG_CLASS,
        'data-message' : finding.message,
        'data-rule'    : finding.code
      }
      if (edit?.before) {
        properties['data-replaced'] = edit.before
      }
      if (edit?.content) {
        properties['data-suggested'] = edit.content
      }
      return {
        end        : { character: finding.end_location.column - 1, line: finding.end_location.row - 1 },
        properties,
        start      : { character: finding.location.column - 1,     line: finding.location.row - 1     }
      }
    })
}

// Binds the corpus findings map at config load, so a fence names its
// fixture with a plain `lint=<id>` token and the decorations are computed
// in preprocess. An id the map does not hold is a build error.
export function lintDecorationTransformer(
  findings: ReadonlyMap<string, readonly LintFinding[]>
): ShikiTransformer {
  return {
    name: 'prose:lint-flag',
    preprocess(code, options) {
      const token = options.meta?.__raw?.split(/\s+/).find(part => part.startsWith(META_PREFIX))
      if (!token) return
      const id    = token.slice(META_PREFIX.length)
      const found = findings.get(id)
      if (!found) throw new Error(`${META_PREFIX}${id} references no fixture findings`)
      ;(options.decorations ??= []).push(...lintDecorations(found, code))
    }
  }
}

export function lintFenceMeta(fixtureId: string): string {
  return META_PREFIX + fixtureId
}
