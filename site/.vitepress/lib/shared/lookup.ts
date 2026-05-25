export function lookup<T>(
  registry : Record<string, T>,
  key      : string,
  label    : string
): T {
  const value = registry[key]
  if (value === undefined) {
    throw new Error(
      `${label} "${key}" not registered. Available: ${Object.keys(registry).sort().join(', ')}`
    )
  }
  return value
}
