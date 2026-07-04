import * as fc from 'fast-check'

// Derives the card-header shorthand for a lint finding from the data the
// hover already carries (rule, flagged text, message, and any fix edit).
// Two shapes cover the lint surface: a `replace` before/after pair and a
// `remove`.

interface RemoveShorthand  { kind: 'remove', text: string }
interface ReplaceShorthand { after: string, before: string, kind: 'replace' }

export type Shorthand = RemoveShorthand | ReplaceShorthand

interface ShorthandInput {
  before    ?: string
  flagged    : string
  message    : string
  rule       : string
  suggested ?: string
}

export function lintShorthand(input: ShorthandInput): Shorthand | null {
  const { before, flagged, message, rule, suggested } = input
  switch (rule) {
    case 'bare-imports':
      // `flagged` spans the import name, so the rewrite needs no message read.
      return flagged
        ? { after: `from ${flagged} import …`, before: `import ${flagged}`, kind: 'replace' }
        : null
    case 'legacy-union-syntax':
      return before !== undefined && suggested !== undefined
        ? { after: truncate(suggested), before, kind: 'replace' }
        : null
    case 'reassigned-constants': {
      // The diagnostic spans the whole assignment, so the name comes from
      // the first backtick of the message, with the lowercase rename standing
      // in for the rule's first suggestion.
      const name = /`([^`]+)`/.exec(message)?.[1]
      return name === undefined ? null : { after: name.toLowerCase(), before: name, kind: 'replace' }
    }
    case 'single-use-variables': {
      // `flagged` spans the binding name, leaving the inlined value to come
      // from the rule's "Consider inlining `<value>`" message.
      const inlined = /Consider inlining `([^`]+)`/.exec(message)?.[1]
      return flagged && inlined !== undefined
        ? { after: truncate(inlined), before: flagged, kind: 'replace' }
        : null
    }
    case 'step-narration':
      return { kind: 'remove', text: truncate(flagged) }
    default:
      return null
  }
}

function truncate(value: string, max = 48): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  type Case = {
    expected : ReturnType<typeof lintShorthand>
    input    : Parameters<typeof lintShorthand>[0]
    name     : string
  }

  describe('lintShorthand', () => {
    test.each<Case>([
      { name: 'bare-imports rewrites the import into the from form',
        input: { flagged: 'os', message: '', rule: 'bare-imports' },
        expected: { after: 'from os import …', before: 'import os', kind: 'replace' } },
      { name: 'bare-imports with no flagged span yields null',
        input: { flagged: '', message: '', rule: 'bare-imports' },
        expected: null },
      { name: 'legacy-union-syntax pairs before with the suggestion',
        input: { before: 'Optional[int]', flagged: '', message: '', rule: 'legacy-union-syntax', suggested: 'int | None' },
        expected: { after: 'int | None', before: 'Optional[int]', kind: 'replace' } },
      { name: 'legacy-union-syntax missing a suggestion yields null',
        input: { before: 'Optional[int]', flagged: '', message: '', rule: 'legacy-union-syntax' },
        expected: null },
      { name: 'reassigned-constants lowercases the name from the message',
        input: { flagged: '', message: 'Constant `MAX` reassigned', rule: 'reassigned-constants' },
        expected: { after: 'max', before: 'MAX', kind: 'replace' } },
      { name: 'reassigned-constants with no backticked name yields null',
        input: { flagged: '', message: 'no name here', rule: 'reassigned-constants' },
        expected: null },
      { name: 'single-use-variables inlines the value from the message',
        input: { flagged: 'x', message: 'Consider inlining `y + 1`', rule: 'single-use-variables' },
        expected: { after: 'y + 1', before: 'x', kind: 'replace' } },
      { name: 'single-use-variables with no inlining hint yields null',
        input: { flagged: 'x', message: 'nope', rule: 'single-use-variables' },
        expected: null },
      { name: 'single-use-variables with no flagged span yields null',
        input: { flagged: '', message: 'Consider inlining `y`', rule: 'single-use-variables' },
        expected: null },
      { name: 'step-narration removes the flagged narration',
        input: { flagged: '# step 1', message: '', rule: 'step-narration' },
        expected: { kind: 'remove', text: '# step 1' } },
      { name: 'an unrecognized rule yields null',
        input: { flagged: 'x', message: '', rule: 'mystery' },
        expected: null }
    ])('$name', ({ expected, input }) => {
      expect(lintShorthand(input)).toEqual(expected)
    })

    test('truncates an over-long suggestion to the ellipsis cap', () => {
      const long   = 'x'.repeat(60)
      const result = lintShorthand({ before: 'T', flagged: '', message: '', rule: 'legacy-union-syntax', suggested: long })
      expect(result).toEqual({ after: `${'x'.repeat(47)}…`, before: 'T', kind: 'replace' })
    })

    test('never emits removed text past the ellipsis cap', () => {
      fc.assert(fc.property(fc.string(), text => {
        const result = lintShorthand({ flagged: text, message: '', rule: 'step-narration' })
        expect(result && 'text' in result ? result.text.length : 0).toBeLessThanOrEqual(48)
      }))
    })
  })
}
