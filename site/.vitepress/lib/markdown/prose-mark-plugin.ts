import type MarkdownIt from 'markdown-it'

import { replaceTextTokens } from './token-split'
import { walkBodyInlines }   from './walk'

const PATTERN = /(?<![\w-])([Pp]rose)(?![\w-])/g

export function proseMarkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'prose-mark', state => {
    walkBodyInlines(state, (block, children) => {
      block.children = replaceTextTokens(children, state.Token, PATTERN, (match, child) => {
        const open    = new state.Token('html_inline', '', 0)
        open.content  = '<span class="prose-mark">'
        open.level    = child.level
        const inner   = new state.Token('text', '', 0)
        inner.content = match[1]
        inner.level   = child.level
        const close   = new state.Token('html_inline', '', 0)
        close.content = '</span>'
        close.level   = child.level
        return [open, inner, close]
      })
    })
  })
}
