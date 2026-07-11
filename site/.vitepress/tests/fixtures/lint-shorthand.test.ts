import { fc, test } from '@fast-check/vitest'

import { lintShorthand } from '../../lib/fixtures/lint-shorthand'

describe('lintShorthand', () => {
  it.each([
    [
      { before: 'Optional[int]', flagged: 'Optional[int]', message: '', rule: 'legacy-union-syntax', suggested: 'int | None' },
      { after: 'int | None', before: 'Optional[int]', kind: 'replace' }
    ],
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
    ]
  ])('shapes a $rule finding', (input, expected) => {
    expect(lintShorthand(input)).toEqual(expected)
  })

  it('returns null for an unknown rule', () => {
    expect(lintShorthand({ flagged: 'x', message: '', rule: 'mystery' })).toBeNull()
  })

  it.each([
    ['a bare-imports without a flagged span',            { flagged: '', message: '', rule: 'bare-imports' }],
    ['a legacy-union-syntax missing its suggestion',     { before: 'Optional[int]', flagged: '', message: '', rule: 'legacy-union-syntax' }],
    ['a miscased-constants without a suggested rename',  { flagged: 'max_retries', message: '', rule: 'miscased-constants' }],
    ['a reassigned-constants without a backticked name', { flagged: '', message: 'no name here', rule: 'reassigned-constants' }],
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
      before    : 'Optional[int]',
      flagged   : '',
      message   : '',
      rule      : 'legacy-union-syntax',
      suggested : 'x'.repeat(60)
    })
    expect(result).toEqual({ after: `${'x'.repeat(47)}…`, before: 'Optional[int]', kind: 'replace' })
  })

  test.prop([fc.string()])('never emits removed text past the ellipsis cap', (flagged) => {
    const result = lintShorthand({ flagged, message: '', rule: 'step-narration' })
    expect(result && 'text' in result ? result.text.length : Infinity).toBeLessThanOrEqual(48)
  })
})
