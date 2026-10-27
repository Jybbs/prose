import type MarkdownIt from 'markdown-it'

import { BODY_LINK_CLASS } from '../shared/constants'
import { walkBodyInlines } from './walk'

export function bodyLinkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'body-link-decorate', state => {
    walkBodyInlines(state, (_block, children) => {
      for (const child of children) {
        if (child.type !== 'link_open') continue
        const existing = child.attrGet('class')
        child.attrSet('class', existing ? `${existing} ${BODY_LINK_CLASS}` : BODY_LINK_CLASS)
      }
    })
  })
}
