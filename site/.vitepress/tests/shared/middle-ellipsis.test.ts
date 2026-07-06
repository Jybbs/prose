import { fc, test } from '@fast-check/vitest'

import { middleEllipsis } from '../../lib/shared/middle-ellipsis'

const fitsWithin = (max: number) => (candidate: string): boolean => candidate.length <= max

describe('middleEllipsis', () => {
  it.each([
    ['keeps text that already fits',             10, 3, 'short',      'short'],
    ['keeps text too short to elide',            2,  3, 'abcd',       'abcd'],
    ['elides the middle and keeps the tail',     7,  3, 'abcdefghij', 'abc…hij'],
    ['drops the whole prefix when nothing fits', 4,  3, 'abcdefghij', '…hij']
  ])('%s', (_name, max, tail, text, expected) => {
    expect(middleEllipsis(fitsWithin(max), tail, text)).toBe(expected)
  })

  test.prop([fc.string({ minLength: 1, maxLength: 60 }), fc.integer({ min: 0, max: 80 })])(
    'never returns more characters than it was given',
    (text, max) => {
      expect(middleEllipsis(fitsWithin(max), 3, text).length).toBeLessThanOrEqual(text.length)
    }
  )

  test.prop([fc.string({ minLength: 1, maxLength: 60 })])(
    'returns the text untouched when the whole string fits',
    (text) => {
      expect(middleEllipsis(() => true, 3, text)).toBe(text)
    }
  )
})
