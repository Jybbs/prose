import * as fc from 'fast-check'

export function formatFolio(n: number, width = 2): string {
  return String(n).padStart(width, '0')
}

export function toRoman(n: number): string {
  const map: Array<[number, string]> = [
    [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
    [100,  'C'], [90,  'XC'], [50,  'L'], [40,  'XL'],
    [10,   'X'], [9,   'IX'], [5,   'V'], [4,   'IV'],
    [1,    'I']
  ]
  let out = ''
  for (const [value, numeral] of map) {
    while (n >= value) {
      out += numeral
      n   -= value
    }
  }
  return out
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('formatFolio', () => {
    test.each([
      [1,   2, '01'],
      [42,  2, '42'],
      [7,   3, '007'],
      [100, 2, '100']
    ])('pads %i to width %i as %s', (n, width, expected) => {
      expect(formatFolio(n, width)).toBe(expected)
    })

    test('pads a single digit to exactly the requested width', () => {
      fc.assert(fc.property(fc.nat({ max: 9 }), fc.integer({ min: 1, max: 4 }), (n, width) => {
        const out = formatFolio(n, width)
        expect(out).toHaveLength(width)
        expect(Number(out)).toBe(n)
      }))
    })
  })

  describe('toRoman', () => {
    test.each([
      [1,    'I'],
      [4,    'IV'],
      [9,    'IX'],
      [40,   'XL'],
      [90,   'XC'],
      [2024, 'MMXXIV'],
      [3888, 'MMMDCCCLXXXVIII']
    ])('converts %i to %s', (n, expected) => {
      expect(toRoman(n)).toBe(expected)
    })
  })
}
