import * as fc from 'fast-check'

// Renders the inline-markdown subset the data-attribute payloads carry
// (backtick code, bold, italics) as HTML, the client-side counterpart to the
// build-time `renderInline`, escaping markup first so a payload never lands
// as live HTML.
export function inlineCode(text: string): string {
  return escapeHtml(text)
    .replaceAll(/`([^`]+)`/g, '<code>$1</code>')
    .replaceAll(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replaceAll(/\*([^*]+)\*/g, '<em>$1</em>')
}

function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('inlineCode', () => {
    test.each([
      { name: 'passes plain text through',        text: 'plain text',       expected: 'plain text' },
      { name: 'wraps backtick code',              text: '`x`',              expected: '<code>x</code>' },
      { name: 'wraps double-asterisk bold',       text: '**y**',            expected: '<strong>y</strong>' },
      { name: 'wraps single-asterisk italics',    text: '*z*',              expected: '<em>z</em>' },
      { name: 'escapes angle brackets',           text: '<script>',         expected: '&lt;script&gt;' },
      { name: 'escapes an ampersand',             text: 'a & b',            expected: 'a &amp; b' },
      { name: 'renders all three inline spans',   text: '`x` **y** *z*',    expected: '<code>x</code> <strong>y</strong> <em>z</em>' }
    ])('$name', ({ text, expected }) => {
      expect(inlineCode(text)).toBe(expected)
    })

    test('leaves markup-free text unchanged', () => {
      fc.assert(fc.property(fc.string().filter(s => !/[<>&*`]/.test(s)), (text) => {
        expect(inlineCode(text)).toBe(text)
      }))
    })
  })
}
