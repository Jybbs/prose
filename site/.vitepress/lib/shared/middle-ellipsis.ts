// Elides the middle of `text` down to the longest prefix the `fits`
// predicate accepts, keeping the last `tail` characters intact.
export function middleEllipsis(
  fits : (candidate: string) => boolean,
  tail : number,
  text : string
): string {
  if (fits(text) || text.length <= tail + 1) return text

  let lo   = 0
  let hi   = text.length - tail - 1
  let best = -1
  while (lo <= hi) {
    const mid = Math.floor((lo + hi) / 2)
    if (fits(`${text.slice(0, mid)}…${text.slice(-tail)}`)) {
      best = mid
      lo   = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return best < 1
    ? `…${text.slice(-tail)}`
    : `${text.slice(0, best)}…${text.slice(-tail)}`
}
