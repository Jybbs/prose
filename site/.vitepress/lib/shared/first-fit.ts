// Seats one item in the first lane whose last end `fits` accepts, opening a new
// lane where none does, records `end` as that lane's last end, and returns the
// lane.
export function firstFit<T>(ends: T[], end: T, fits: (last: T) => boolean): number {
  const free = ends.findIndex(fits)
  const lane = free === -1 ? ends.length : free
  ends[lane] = end
  return lane
}
