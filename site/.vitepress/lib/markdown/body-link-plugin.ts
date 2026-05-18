import type MarkdownIt from 'markdown-it'

import { walkBodyInlines } from './walk'

export function bodyLinkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'body-link-decorate', state => {
    walkBodyInlines(state, block => {
      for (const child of block.children!) {
        if (child.type !== 'link_open') continue
        const existing = child.attrGet('class')
        child.attrSet('class', existing ? `${existing} body-link` : 'body-link')
      }
    })
  })
}
