export async function withFallback<T>(
  label    : string,
  fn       : () => T | Promise<T>,
  fallback : T
): Promise<T> {
  try {
    return await fn()
  }
  catch (err) {
    warnFallback(label, err)
    return fallback
  }
}

export function withFallbackSync<T>(
  label    : string,
  fn       : () => T,
  fallback : T
): T {
  try {
    return fn()
  }
  catch (err) {
    warnFallback(label, err)
    return fallback
  }
}

function warnFallback(label: string, err: unknown): void {
  console.warn(`[data:${label}] external call failed, using fallback:`, err instanceof Error ? err.message : err)
}
