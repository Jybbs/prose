export const ROW_STRIDE_PX = 200

const ROT_STEP = 67

const LETTERS = ['r', 'o', 's', 'e'] as const

const PERMUTATIONS = permutations(LETTERS)

const CORNER_SIGNS: readonly [number, number][] = [[1, -1], [1, 1], [-1, 1], [-1, -1]]

const CORNER_OFFSET = 0.36

const CORNER_DY = ROW_STRIDE_PX * CORNER_OFFSET

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

type Stamp = BigStamp | SmallStamp

// Returns every ordering of `items`, the first element varying outermost
// while the rest keep their relative order.
function permutations<T>(items: readonly T[]): T[][] {
  if (items.length <= 1) return [[...items]]
  return items.flatMap((item, index) =>
    permutations(items.toSpliced(index, 1)).map(rest => [item, ...rest]))
}

function rotate(idx: number): number {
  return ((idx * ROT_STEP) % 360) - 180
}

// Tiles the hero watermark field, emitting one big pilcrow and four
// hash-seeded corner letters per cell. A `(cols, rows)` pair maps to a
// deterministic `Stamp` array the component renders.
export function tileStamps(cols: number, rows: number): readonly Stamp[] {
  const out: Stamp[] = []
  const dx  = (100 / cols) * CORNER_OFFSET
  let   idx = 0
  for (let r = 0; r < rows; r++) {
    for (let cIdx = 0; cIdx < cols; cIdx++) {
      const xC = ((cIdx + 0.5) / cols) * 100
      const yC = (r + 0.5) * ROW_STRIDE_PX
      out.push({ kind: 'big', rotate: rotate(idx), x: xC, y: yC })
      idx++
      const cellSeed = ((r * 2654435761) ^ (cIdx * 40503)) >>> 0
      const shuffled = PERMUTATIONS[cellSeed % PERMUTATIONS.length]
      for (const [i, [sx, sy]] of CORNER_SIGNS.entries()) {
        out.push({
          kind   : 'small',
          letter : shuffled[i],
          rotate : rotate(idx),
          x      : xC + sx * dx,
          y      : yC + sy * CORNER_DY
        })
        idx++
      }
    }
  }
  return out
}
