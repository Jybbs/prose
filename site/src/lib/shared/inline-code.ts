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
