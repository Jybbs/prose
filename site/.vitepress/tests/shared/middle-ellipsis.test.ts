import { fc, test } from '@fast-check/vitest'

import { middleEllipsis } from '../../lib/shared/middle-ellipsis'

const fitsWithin = (max: number) => (candidate: string): boolean => candidate.length <= max

describe('middleEllipsis', () => {
  it.each([
    { expected: 'short',   max: 10, name: 'keeps text that already fits',             tail: 3, text: 'short'      },
    { expected: 'abcd',    max: 2,  name: 'keeps text too short to elide',            tail: 3, text: 'abcd'       },
    { expected: 'abc…hij', max: 7,  name: 'elides the middle and keeps the tail',     tail: 3, text: 'abcdefghij' },
    { expected: '…hij',    max: 4,  name: 'drops the whole prefix when nothing fits', tail: 3, text: 'abcdefghij' }
  ])('$name', ({ expected, max, tail, text }) => {
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
