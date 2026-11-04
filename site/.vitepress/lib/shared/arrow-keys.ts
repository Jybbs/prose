const STEPS: Record<string, number> = { ArrowDown: 1, ArrowLeft: -1, ArrowRight: 1, ArrowUp: -1 }

// Returns the index the arrow keys, Home, and End move the focus to from `at`
// among `count` items, stopping at either end, or past it to the far end
// where `wrap` is set, and `undefined` for any other key.
export function arrowIndex(key: string, at: number, count: number, wrap = false): number | undefined {
  if (key === 'Home') return 0
  if (key === 'End') return count - 1
  const step = STEPS[key]
  if (step === undefined) return undefined
  return wrap ? (at + step + count) % count : Math.min(Math.max(at + step, 0), count - 1)
}
