import type { DecorationItem, ShikiTransformer } from '@shikijs/types'

import type { LintFinding } from '../fixtures/lint-findings'

const META_PREFIX = 'lintdeco-'

// Encodes decorations into a fence-meta token the transformer decodes in
// its preprocess hook, so they travel inside the markdown string rather
// than through module state the config and build realms do not share.
export function encodeLintMeta(decorations: readonly DecorationItem[]): string {
  return META_PREFIX + Buffer.from(JSON.stringify(decorations)).toString('base64url')
}

export function decodeLintMeta(token: string): DecorationItem[] {
  const json = Buffer.from(token.slice(META_PREFIX.length), 'base64url').toString('utf8')
  return JSON.parse(json) as DecorationItem[]
}

// Converts findings into shiki decorations that wrap each flagged span
// in a `.lint-flag` element carrying the hover data as `data-*`. Sorted
// by position, since shiki rejects unordered or overlapping ranges.
export function lintDecorations(findings: readonly LintFinding[]): DecorationItem[] {
  return findings
    .toSorted((a, b) => a.location.row - b.location.row || a.location.column - b.location.column)
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

export const lintDecorationTransformer: ShikiTransformer = {
  name: 'prose:lint-flag',
  preprocess(_code, options) {
    const token = options.meta?.__raw?.split(/\s+/).find(part => part.startsWith(META_PREFIX))
    if (!token) return
    ;(options.decorations ??= []).push(...decodeLintMeta(token))
  }
}
