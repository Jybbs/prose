interface EvenRowOptions {
  available : number
  gap       : number
  minWidth  : number
}

// Splits a roster across as few rows as keep every entry at or above `minWidth`,
// then levels the rows so none carries more than one entry over another. An
// `available` of zero reads as unmeasured and holds every entry on one row.
export function evenRows<T>(items: readonly T[], options: EvenRowOptions): T[][] {
  const { available, gap, minWidth } = options
  if (items.length === 0) return []

  const perRowCap = available > 0
    ? Math.max(1, Math.floor((available + gap) / (minWidth + gap)))
    : items.length
  const rowCount  = Math.max(1, Math.ceil(items.length / perRowCap))
  const base      = Math.floor(items.length / rowCount)
  const remainder = items.length % rowCount

  return Array.from({ length: rowCount }, (_, row) => {
    const start = row * base + Math.min(row, remainder)
    return items.slice(start, start + base + (row < remainder ? 1 : 0))
  })
}
