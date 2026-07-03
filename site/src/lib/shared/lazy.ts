// Memoizes a zero-argument derivation, deriving on the first call and
// returning the cached value on every later one.
export function lazy<T>(derive: () => T): () => T {
  let cached: T | undefined
  return () => (cached ??= derive())
}
