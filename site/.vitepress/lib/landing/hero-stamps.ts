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

// Tiles the hero watermark field: each cell emits one big pilcrow plus four
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
      const cellSeed = ((r * 2654435761) ^ (cIdx * 40503)) >>> 0
      const shuffled = PERMUTATIONS[cellSeed % PERMUTATIONS.length]
      for (let i = 0; i < 4; i++) {
        const [sx, sy] = CORNER_SIGNS[i]
        out.push({ kind: 'small', letter: shuffled[i], rotate: rotate(idx), x: xC + sx * dx, y: yC + sy * dy })
        idx++
      }
    }
  }
  return out
}
