// The number of leading elements two sequences share, counting characters
// for a string and entries for an array.
export function commonPrefix<T>(a: ArrayLike<T>, b: ArrayLike<T>): number {
  const max = Math.min(a.length, b.length)
  let count = 0
  while (count < max && a[count] === b[count]) count += 1
  return count
}
