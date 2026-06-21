export function memoizeByPath<T>(compute: (dir: string) => T): (dir: string) => T {
  const cache = new Map<string, T>()
  return dir => {
    const cached = cache.get(dir)
    if (cached !== undefined) return cached
    const value = compute(dir)
    cache.set(dir, value)
    return value
  }
}
