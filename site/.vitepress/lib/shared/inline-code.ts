// Renders inline-backtick prose as `<code>` spans, the client-side
// counterpart to the build-time `md.renderInline`.
export function inlineCode(text: string): string {
  return text.replaceAll(/`([^`]+)`/g, '<code>$1</code>')
}
