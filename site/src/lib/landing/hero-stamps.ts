import * as fc from 'fast-check'

export const ROW_STRIDE_PX = 200

const ROT_STEP = 67

const PERMUTATIONS: readonly (readonly string[])[] = [
  ['r','o','s','e'], ['r','o','e','s'], ['r','s','o','e'], ['r','s','e','o'],
  ['r','e','o','s'], ['r','e','s','o'], ['o','r','s','e'], ['o','r','e','s'],
  ['o','s','r','e'], ['o','s','e','r'], ['o','e','r','s'], ['o','e','s','r'],
  ['s','r','o','e'], ['s','r','e','o'], ['s','o','r','e'], ['s','o','e','r'],
  ['s','e','r','o'], ['s','e','o','r'], ['e','r','o','s'], ['e','r','s','o'],
  ['e','o','r','s'], ['e','o','s','r'], ['e','s','r','o'], ['e','s','o','r']
]

const CORNER_SIGNS: readonly [number, number][] = [[1, -1], [1, 1], [-1, 1], [-1, -1]]

interface BigStamp {
  kind   : 'big'
  rotate : number
  x      : number
  y      : number
}

interface SmallStamp {
  kind   : 'small'
  letter : string
  rotate : number
  x      : number
  y      : number
}

export type Stamp = BigStamp | SmallStamp

function rotate(idx: number): number {
  return ((idx * ROT_STEP) % 360) - 180
}

// Tiles the hero watermark field, each cell emitting one big pilcrow plus four
// hash-seeded corner letters, so a (cols, rows) pair maps to a deterministic
// Stamp array the component renders.
export function tileStamps(cols: number, rows: number): readonly Stamp[] {
  const out: Stamp[] = []
  let idx = 0
  for (let r = 0; r < rows; r++) {
    for (let cIdx = 0; cIdx < cols; cIdx++) {
      const xC = ((cIdx + 0.5) / cols) * 100
      const yC = (r + 0.5) * ROW_STRIDE_PX
      out.push({ kind: 'big', rotate: rotate(idx), x: xC, y: yC })
      idx++
      const o        = 0.36
      const dx       = (100 / cols) * o
      const dy       = ROW_STRIDE_PX * o
      // The unsigned shift keeps the XOR seed non-negative, which `Math.trunc` would not.
      // oxlint-disable-next-line unicorn/prefer-math-trunc
      const cellSeed = ((r * 2654435761) ^ (cIdx * 40503)) >>> 0
      const shuffled = PERMUTATIONS[cellSeed % PERMUTATIONS.length]
      for (const [i, [sx, sy]] of CORNER_SIGNS.entries()) {
        out.push({ kind: 'small', letter: shuffled[i], rotate: rotate(idx), x: xC + sx * dx, y: yC + sy * dy })
        idx++
      }
    }
  }
  return out
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const LETTERS = new Set(['e', 'o', 'r', 's'])

  describe('tileStamps', () => {
    test('renders one big pilcrow and four corner letters per cell', () => {
      expect(tileStamps(1, 1)).toEqual([
        { kind: 'big',                rotate: -180, x: 50, y: 100 },
        { kind: 'small', letter: 'r', rotate: -113, x: 86, y: 28  },
        { kind: 'small', letter: 'o', rotate: -46,  x: 86, y: 172 },
        { kind: 'small', letter: 's', rotate: 21,   x: 14, y: 172 },
        { kind: 'small', letter: 'e', rotate: 88,   x: 14, y: 28  }
      ])
    })

    test('emits five stamps per cell with valid letters and rotations', () => {
      fc.assert(fc.property(fc.integer({ min: 1, max: 8 }), fc.integer({ min: 1, max: 8 }), (cols, rows) => {
        const stamps = tileStamps(cols, rows)
        expect(stamps).toHaveLength(cols * rows * 5)
        expect(stamps.filter(stamp => stamp.kind === 'big')).toHaveLength(cols * rows)
        for (const stamp of stamps) {
          expect(stamp.rotate).toBeGreaterThanOrEqual(-180)
          expect(stamp.rotate).toBeLessThan(180)
          if (stamp.kind === 'small') expect(LETTERS.has(stamp.letter)).toBe(true)
        }
      }))
    })

    test('is deterministic for a given grid', () => {
      fc.assert(fc.property(fc.integer({ min: 1, max: 6 }), fc.integer({ min: 1, max: 6 }), (cols, rows) => {
        expect(tileStamps(cols, rows)).toEqual(tileStamps(cols, rows))
      }))
    })
  })
}
