import type MarkdownIt from 'markdown-it'

import { replaceTextTokens } from './token-split'
import { walkBodyInlines }   from './walk'
import { wordBounded }       from './word-bounded'

const PATTERN = wordBounded('[Pp]rose')

export function proseMarkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'prose-mark', state => {
    walkBodyInlines(state, (block, children) => {
      block.children = replaceTextTokens(children, state.Token, PATTERN, (match, child) => {
        const open    = new state.Token('span_open', 'span', 1)
        open.attrSet('class', 'prose-mark')
        open.level    = child.level
        const inner   = new state.Token('text', '', 0)
        inner.content = match[1]
        inner.level   = child.level + 1
        const close   = new state.Token('span_close', 'span', -1)
        close.level   = child.level
        return [open, inner, close]
      })
    })
  })
}
