// Returns `value` narrowed non-nullish, throwing `message` so a broken
// content reference fails the build where it was made.
export function required<T>(value: T | null | undefined, message: string): T {
  if (value === null || value === undefined) throw new Error(message)
  return value
}
