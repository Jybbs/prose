import * as fc from 'fast-check'

export function middleEllipsis(
  fits : (candidate: string) => boolean,
  tail : number,
  text : string
): string {
  if (fits(text)) return text
  if (text.length <= tail + 1) return text

  let lo   = 0
  let hi   = text.length - tail - 1
  let best = -1
  while (lo <= hi) {
    const mid = Math.floor((lo + hi) / 2)
    if (fits(`${text.slice(0, mid)}…${text.slice(-tail)}`)) {
      best = mid
      lo   = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return best < 1
    ? `…${text.slice(-tail)}`
    : `${text.slice(0, best)}…${text.slice(-tail)}`
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const fitsWithin = (max: number) => (candidate: string): boolean => candidate.length <= max

  describe('middleEllipsis', () => {
    test.each([
      { name: 'keeps text that already fits',            max: 10, tail: 3, text: 'short',      expected: 'short' },
      { name: 'keeps text too short to elide',           max: 2,  tail: 3, text: 'abcd',       expected: 'abcd' },
      { name: 'elides the middle and keeps the tail',    max: 7,  tail: 3, text: 'abcdefghij', expected: 'abc…hij' },
      { name: 'drops the whole prefix when nothing fits', max: 4,  tail: 3, text: 'abcdefghij', expected: '…hij' }
    ])('$name', ({ max, tail, text, expected }) => {
      expect(middleEllipsis(fitsWithin(max), tail, text)).toBe(expected)
    })

    test('never returns more characters than it was given', () => {
      fc.assert(fc.property(fc.string({ minLength: 1, maxLength: 60 }), fc.integer({ min: 0, max: 80 }), (text, max) => {
        expect(middleEllipsis(fitsWithin(max), 3, text).length).toBeLessThanOrEqual(text.length)
      }))
    })

    test('returns the text untouched when the whole string fits', () => {
      fc.assert(fc.property(fc.string({ minLength: 1, maxLength: 60 }), (text) => {
        expect(middleEllipsis(() => true, 3, text)).toBe(text)
      }))
    })
  })
}
