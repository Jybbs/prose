// Returns `value` narrowed non-nullish, throwing `message` so a broken
// content reference fails the build where it was made.
export function required<T>(value: T | null | undefined, message: string): T {
  if (value === null || value === undefined) throw new Error(message)
  return value
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('required', () => {
    test.each([
      { name: 'returns a non-empty value', value: 'x' },
      { name: 'returns zero',              value: 0 },
      { name: 'returns an empty string',   value: '' },
      { name: 'returns false',             value: false }
    ])('$name', ({ value }) => {
      expect(required(value, 'unreachable')).toBe(value)
    })

    test.each([
      { name: 'throws on null',      value: null },
      { name: 'throws on undefined', value: undefined }
    ])('$name', ({ value }) => {
      expect(() => required(value, 'missing content reference')).toThrow('missing content reference')
    })
  })
}
