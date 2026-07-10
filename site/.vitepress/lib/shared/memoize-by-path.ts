export function memoizeByPath<T>(compute: (dir: string) => T): (dir: string) => T {
  const cache = new Map<string, T>()
  return dir => cache.getOrInsertComputed(dir, compute)
}
