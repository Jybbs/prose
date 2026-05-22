export function inlineCodeHtml(text: string): string {
  return text.replace(/`([^`]+)`/g, '<code>$1</code>')
}
