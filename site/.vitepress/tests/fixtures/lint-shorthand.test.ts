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

  it('returns null when a replace lacks its operands', () => {
    expect(lintShorthand({ flagged: '', message: '', rule: 'bare-imports' })).toBeNull()
  })

  it('truncates a long remove to 48 chars with an ellipsis', () => {
    const flagged = 'x'.repeat(60)
    expect(lintShorthand({ flagged, message: '', rule: 'step-narration' }))
      .toEqual({ kind: 'remove', text: `${'x'.repeat(47)}…` })
  })
})
