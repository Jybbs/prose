import { escapeHtml } from './escape-html'

const CODE_SPAN = /`([^`]+)`/g

// Escapes the text and then wraps each backtick span in `<code>`, the two
// forms a fixture title or a diagnostic message carries. An intraword
// underscore in a name like `SCREAMING_CASE` stays as written.
export function inlineCode(text: string): string {
  return escapeHtml(text).replaceAll(CODE_SPAN, '<code>$1</code>')
}
