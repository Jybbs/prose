import * as fc from 'fast-check'

// Serializes a value for a `<script type="application/json">` island payload,
// escaping `<` so no token sequence can close the carrying script element.
export function embedJson(value: unknown): string {
  return JSON.stringify(value).replaceAll('<', '\\u003c')
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('embedJson', () => {
    test.each([
      { name: 'serializes a plain object',        value: { a: 1 },      expected: '{"a":1}' },
      { name: 'escapes an angle bracket',         value: '<script>',    expected: '"\\u003cscript>"' },
      { name: 'escapes a bracket inside an array', value: ['a<b'],      expected: '["a\\u003cb"]' }
    ])('$name', ({ value, expected }) => {
      expect(embedJson(value)).toBe(expected)
    })

    test('never emits a raw `<` and round-trips through JSON.parse', () => {
      fc.assert(fc.property(fc.string(), (text) => {
        const payload = embedJson(text)
        expect(payload).not.toContain('<')
        expect(JSON.parse(payload)).toBe(text)
      }))
    })

    test('round-trips a record and escapes every bracket', () => {
      fc.assert(fc.property(fc.dictionary(fc.string(), fc.oneof(fc.string(), fc.integer(), fc.boolean())), (record) => {
        const payload = embedJson(record)
        expect(payload).not.toContain('<')
        expect(JSON.parse(payload)).toEqual(record)
      }))
    })
  })
}
