export function lookup<T>(
  registry : Record<string, T> | ReadonlyMap<string, T>,
  key      : string,
  label    : string
): T {
  const isMap = registry instanceof Map
  const value = isMap ? registry.get(key) : (registry as Record<string, T>)[key]
  if (value === undefined) {
    const available = isMap
      ? [...registry.keys()].sort()
      : Object.keys(registry).sort()
    throw new Error(
      `${label} "${key}" not registered. Available: ${available.join(', ')}`
    )
  }
  return value
}
