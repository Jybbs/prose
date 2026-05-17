export async function withFallback<T>(
  label    : string,
  fn       : () => T | Promise<T>,
  fallback : T
): Promise<T> {
  try {
    return await fn()
  }
  catch (err) {
    console.warn(`[data:${label}] external call failed, using fallback:`, err instanceof Error ? err.message : err)
    return fallback
  }
}
