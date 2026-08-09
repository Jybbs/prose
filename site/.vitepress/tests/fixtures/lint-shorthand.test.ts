import { fc, test } from '@fast-check/vitest'

import { readLintFindings, type LintFinding } from '../../lib/fixtures/lint-findings'
import { lintShorthand }                      from '../../lib/fixtures/lint-shorthand'
import { readFixtureToggle }                  from '../../lib/fixtures/toggle'
import { walkFixtures }                       from '../../lib/fixtures/walker'
import { crateDir }                           from '../../lib/shared/paths'

const SPLIT = '(\n    "the quick brown fox jumps over "\n    "the lazy dog"\n)'

const overflow = (suggested: string, replaced = '"x"') =>
  lintShorthand({ flagged: 'x', message: '', replaced, rule: 'line-overflow', suggested })

// The source a finding's span covers, which is what the popover reads off
// the decorated element as its flagged text.
function flaggedText(finding: LintFinding, lines: readonly string[]): string {
  const span = lines.slice(finding.location.row - 1, finding.end_location.row)
  if (span.length === 0) return ''
  const last = span.length - 1
  span[last] = [...span[last]].slice(0, finding.end_location.column - 1).join('')
  span[0]    = [...span[0]].slice(finding.location.column - 1).join('')
  return span.join('\n')
}

describe('lintShorthand', () => {
  it.each([
    [
      { flagged: 'tmp', message: 'Consider inlining `compute()`', rule: 'single-use-variables' },
      { after: 'compute()', before: 'tmp', kind: 'replace' }
    ],
    [
      { flagged: 'numpy', message: '', rule: 'bare-imports' },
      { after: 'from numpy import …', before: 'import numpy', kind: 'replace' }
    ],
    [
      { flagged: 'max_retries', message: '', rule: 'miscased-constants', suggested: 'MAX_RETRIES' },
      { after: 'MAX_RETRIES', before: 'max_retries', kind: 'replace' }
    ],
    [
      { flagged: 'MAX = 5', message: 'Constant `MAX` reassigned', rule: 'reassigned-constants' },
      { after: 'max', before: 'MAX', kind: 'replace' }
    ],
    [
      { flagged: '# Step 1: parse', message: '', rule: 'step-narration' },
      { kind: 'remove', text: '# Step 1: parse' }
    ],
    [
      { flagged: 'count', message: '', rule: 'signature-annotations', suggested: ': int' },
      { anchor: 'count', inserted: ': int', kind: 'insert' }
    ],
    [
      {
        flagged   : 'BANNER = "the quick brown fox jumps over the lazy dog"',
        message   : '',
        replaced  : '"the quick brown fox jumps over the lazy dog"',
        rule      : 'line-overflow',
        suggested : SPLIT
      },
      { after: SPLIT, before: '"the quick brown fox jumps over the lazy dog"', kind: 'block' }
    ]
  ])('shapes a $rule finding', (input, expected) => {
    expect(lintShorthand(input)).toEqual(expected)
  })

  it('returns null for an unknown rule', () => {
    expect(lintShorthand({ flagged: 'x', message: '', rule: 'mystery' })).toBeNull()
  })

  it.each([
    ['a bare-imports without a flagged span',            { flagged: '', message: '', rule: 'bare-imports' }],
    ['a line-overflow without a replaced literal',       { flagged: 'x', message: '', rule: 'line-overflow', suggested: SPLIT }],
    ['a line-overflow without a suggested split',        { flagged: 'x', message: '', replaced: '"x"', rule: 'line-overflow' }],
    ['a miscased-constants without a suggested rename',  { flagged: 'max_retries', message: '', rule: 'miscased-constants' }],
    ['a reassigned-constants without a backticked name', { flagged: '', message: 'no name here', rule: 'reassigned-constants' }],
    ['a signature-annotations without an annotation',    { flagged: 'count', message: '', rule: 'signature-annotations' }],
    ['a signature-annotations without a flagged span',   { flagged: '', message: '', rule: 'signature-annotations', suggested: ': int' }],
    ['a single-use-variables without an inlining hint',  { flagged: 'x', message: 'nope', rule: 'single-use-variables' }],
    ['a single-use-variables without a flagged span',    { flagged: '', message: 'Consider inlining `y`', rule: 'single-use-variables' }]
  ])('returns null for %s', (_name, input) => {
    expect(lintShorthand(input)).toBeNull()
  })

  it('truncates a long remove to 48 chars with an ellipsis', () => {
    const flagged = 'x'.repeat(60)
    expect(lintShorthand({ flagged, message: '', rule: 'step-narration' }))
      .toEqual({ kind: 'remove', text: `${'x'.repeat(47)}…` })
  })

  it('truncates an over-long suggestion to the same cap', () => {
    const result = lintShorthand({
      flagged   : 'max_retries',
      message   : '',
      rule      : 'miscased-constants',
      suggested : 'x'.repeat(60)
    })
    expect(result).toEqual({ after: `${'x'.repeat(47)}…`, before: 'max_retries', kind: 'replace' })
  })

  it('keeps the chip pair when the suggested split fits one line', () => {
    expect(overflow('"a" "b"', '"ab"')).toEqual({ after: '"a" "b"', before: '"ab"', kind: 'replace' })
  })

  it('takes the block shape when only the replaced side spans lines', () => {
    expect(overflow('c', 'a\nb')).toEqual({ after: 'c', before: 'a\nb', kind: 'block' })
  })

  it('caps the stacked panes at ten lines with an elision marker', () => {
    const parts = Array.from({ length: 14 }, (_, index) => `    "part ${index}"`).join('\n')
    expect(overflow(parts)).toEqual({
      after  : `${parts.split('\n').slice(0, 10).join('\n')}\n…`,
      before : '"x"',
      kind   : 'block'
    })
  })

  test.prop([fc.integer({ min: 1, max: 40 })])('never emits a pane past the line cap', (count) => {
    const result = overflow(Array.from({ length: count }, (_, index) => `    "${index}"`).join('\n'))
    expect(result && 'after' in result ? result.after.split('\n').length : 0).toBeLessThanOrEqual(11)
  })

  test.prop([fc.string()])('never emits removed text past the ellipsis cap', (flagged) => {
    const result = lintShorthand({ flagged, message: '', rule: 'step-narration' })
    expect(result && 'text' in result ? result.text.length : Infinity).toBeLessThanOrEqual(48)
  })
})

// The fixture corpus records which rules carry a display-only fix, so a rule
// reaching that state without a case above fails here rather than falling to
// the popover's plain message.
describe('display-only fix coverage', () => {
  it('shapes a shorthand for every corpus finding carrying a display-only fix', async () => {
    const unshaped: string[] = []
    let shaped = 0

    for (const { id, inputPath } of walkFixtures(crateDir(import.meta.url))) {
      const display = readLintFindings(inputPath)
        .filter(finding => finding.fix?.applicability === 'displayonly')
      if (display.length === 0) continue
      const lines = (await readFixtureToggle(inputPath)).output.split('\n')

      for (const finding of display) {
        const edit = finding.fix?.edits[0]
        const shorthand = lintShorthand({
          flagged   : flaggedText(finding, lines),
          message   : finding.message,
          replaced  : edit?.before || undefined,
          rule      : finding.code,
          suggested : edit?.content
        })
        if (shorthand) shaped += 1
        else unshaped.push(`${id} (${finding.code})`)
      }
    }

    expect(unshaped).toEqual([])
    expect(shaped).toBeGreaterThan(0)
  })
})
