import type { MarkdownIt } from 'markdown-it'

// Gives each table a tab stop, since the default theme lets a table scroll
// across, so a keyboard scrolls it at any width.
export function scrollStopPlugin(md: MarkdownIt): void {
  md.core.ruler.push('scroll-stop', state => {
    for (const token of state.tokens) {
      if (token.type === 'table_open') token.attrSet('tabindex', '0')
    }
  })
}
