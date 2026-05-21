import type MarkdownIt from 'markdown-it'

import { walkBodyInlines } from './walk'

const PATTERN = /(?<![\w-])([Pp]rose)(?![\w-])/g

type Token = MarkdownIt.Token

export function proseMarkPlugin(md: MarkdownIt): void {
  md.core.ruler.after('inline', 'prose-mark', state => {
    walkBodyInlines(state, (block, children) => {
      block.children = decorateChildren(children, state.Token)
    })
  })
}

function decorateChildren(
  children : Token[],
  TokenCtor: new (type: string, tag: string, nesting: MarkdownIt.Token.Nesting) => Token
): Token[] {
  const out: Token[] = []
  for (const child of children) {
    if (child.type !== 'text') {
      out.push(child)
      continue
    }
    const decorated = decorateText(child.content, TokenCtor, child.level)
    if (decorated.length === 1 && decorated[0].content === child.content) {
      out.push(child)
    }
    else {
      out.push(...decorated)
    }
  }
  return out
}

function decorateText(
  text     : string,
  TokenCtor: new (type: string, tag: string, nesting: MarkdownIt.Token.Nesting) => Token,
  level    : number
): Token[] {
  PATTERN.lastIndex = 0
  const out: Token[] = []
  let   cursor       = 0
  let   match: RegExpExecArray | null

  while ((match = PATTERN.exec(text)) !== null) {
    if (match.index > cursor) {
      const leading   = new TokenCtor('text', '', 0)
      leading.content = text.slice(cursor, match.index)
      leading.level   = level
      out.push(leading)
    }

    const open       = new TokenCtor('html_inline', '', 0)
    open.content     = '<span class="prose-mark">'
    open.level       = level
    out.push(open)

    const inner     = new TokenCtor('text', '', 0)
    inner.content   = match[1]
    inner.level     = level
    out.push(inner)

    const close      = new TokenCtor('html_inline', '', 0)
    close.content    = '</span>'
    close.level      = level
    out.push(close)

    cursor = match.index + match[1].length
  }

  if (out.length === 0) {
    const verbatim   = new TokenCtor('text', '', 0)
    verbatim.content = text
    verbatim.level   = level
    return [verbatim]
  }

  if (cursor < text.length) {
    const trailing   = new TokenCtor('text', '', 0)
    trailing.content = text.slice(cursor)
    trailing.level   = level
    out.push(trailing)
  }

  return out
}
