// Memoizes a zero-argument derivation, deriving on the first call and
// returning the cached value on every later one.
export function lazy<T>(derive: () => T): () => T {
  let cached: T | undefined
  return () => (cached ??= derive())
}

if (import.meta.vitest) {
  const { describe, expect, test, vi } = import.meta.vitest

  describe('lazy', () => {
    test('derives once and returns the cached reference thereafter', () => {
      const derive = vi.fn(() => ({ token: 'value' }))
      const get    = lazy(derive)

      const first = get()
      expect(get()).toBe(first)
      expect(get()).toBe(first)
      expect(derive).toHaveBeenCalledOnce()
    })

    test('caches a falsy-but-defined value rather than re-deriving', () => {
      const derive = vi.fn(() => 0)
      const get    = lazy(derive)

      expect(get()).toBe(0)
      expect(get()).toBe(0)
      expect(derive).toHaveBeenCalledOnce()
    })
  })
}
