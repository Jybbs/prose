import MarkdownIt from 'markdown-it'

const md = new MarkdownIt()

export function inlineCode(text: string): string {
  return md.renderInline(text)
}
