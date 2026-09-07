// Groups `items` by a key drawn from a closed registry, filling every member
// the key never produced with an empty list.
export function groupByMember<T, K extends string>(
  items   : readonly T[],
  key     : (item: T) => K,
  members : readonly K[]
): Record<K, readonly T[]> {
  const out = Object.groupBy(items, key) as Record<K, readonly T[]>
  for (const member of members) out[member] ??= []
  return out
}
