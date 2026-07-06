import type { DecorationItem, ShikiTransformer } from '@shikijs/types'

import type { LintFinding } from '../fixtures/lint-findings'

const META_PREFIX = 'lint='

// Converts findings into shiki decorations that wrap each flagged span
// in a `.lint-flag` element carrying the hover data as `data-*`. Sorted
// by position, since shiki rejects unordered or overlapping ranges.
export function lintDecorations(findings: readonly LintFinding[]): DecorationItem[] {
  return findings
    .toSorted((a, b) => a.location.row - b.location.row || a.location.column - b.location.column)
    .map(finding => {
      const properties: Record<string, string> = {
        class          : 'lint-flag underline-draw',
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

// Binds the corpus findings map at config load, so a fence names its
// fixture with a plain `lint=<id>` token and the decorations are computed
// in preprocess. An id the map does not hold is a build error.
export function lintDecorationTransformer(
  findings: ReadonlyMap<string, readonly LintFinding[]>
): ShikiTransformer {
  return {
    name: 'prose:lint-flag',
    preprocess(_code, options) {
      const token = options.meta?.__raw?.split(/\s+/).find(part => part.startsWith(META_PREFIX))
      if (!token) return
      const id    = token.slice(META_PREFIX.length)
      const found = findings.get(id)
      if (!found) throw new Error(`${META_PREFIX}${id} references no fixture findings`)
      ;(options.decorations ??= []).push(...lintDecorations(found))
    }
  }
}

export function lintFenceMeta(fixtureId: string): string {
  return META_PREFIX + fixtureId
}
