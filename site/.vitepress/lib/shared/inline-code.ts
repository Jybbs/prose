// Renders inline-backtick prose as `<code>` spans, the client-side
// counterpart to the build-time `md.renderInline`, escaping markup before
// the backtick pass.
export function inlineCode(text: string): string {
  return escapeHtml(text).replaceAll(/`([^`]+)`/g, '<code>$1</code>')
}

function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
}
