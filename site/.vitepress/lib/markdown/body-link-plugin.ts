import type MarkdownIt from 'markdown-it'

import { walkBodyInlines } from './walk'

export function bodyLinkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'body-link-decorate', state => {
    walkBodyInlines(state, (_block, children) => {
      for (const child of children) {
        if (child.type !== 'link_open') continue
        const existing = child.attrGet('class')
        child.attrSet('class', existing ? `${existing} body-link` : 'body-link')
      }
    })
  })
}
