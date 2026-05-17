import type MarkdownIt from 'markdown-it'

export function bodyLinkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'body-link-decorate', state => {
    for (let i = 0; i < state.tokens.length; i++) {
      const block = state.tokens[i]
      if (block.type !== 'inline' || !block.children) continue
      const prev = state.tokens[i - 1]
      if (prev?.type.startsWith('heading_')) continue
      for (const child of block.children) {
        if (child.type !== 'link_open') continue
        const existing = child.attrGet('class')
        child.attrSet('class', existing ? `${existing} body-link` : 'body-link')
      }
    }
  })
}
