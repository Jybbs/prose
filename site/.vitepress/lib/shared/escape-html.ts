// Replaces `&`, `<`, `>`, and `"` with their HTML entities, so a string
// interpolated into `v-html` renders as text rather than as markup.
export function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}
