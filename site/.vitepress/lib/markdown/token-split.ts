import type MarkdownIt from 'markdown-it'

type Token     = MarkdownIt.Token
type TokenCtor = new (type: string, tag: string, nesting: MarkdownIt.Token.Nesting) => Token

function splitTextToken(
  child    : Token,
  pattern  : RegExp,
  TokenCtor: TokenCtor,
  replace  : (match: RegExpExecArray) => Token[] | null
): Token | Token[] {
  const text = child.content
  pattern.lastIndex = 0
  const out: Token[] = []
  let   cursor       = 0
  let   match: RegExpExecArray | null

  while ((match = pattern.exec(text)) !== null) {
    const replacement = replace(match)
    if (!replacement) continue

    if (match.index > cursor) {
      out.push(makeText(text.slice(cursor, match.index), child.level, TokenCtor))
    }
    out.push(...replacement)
    cursor = match.index + match[0].length
  }

  if (out.length === 0) return child
  if (cursor < text.length) {
    out.push(makeText(text.slice(cursor), child.level, TokenCtor))
  }
  return out
}

export function replaceTextTokens(
  children : Token[],
  TokenCtor: TokenCtor,
  pattern  : RegExp,
  replace  : (match: RegExpExecArray, child: Token) => Token[] | null,
  options  : { skipInsideLinks?: boolean } = {}
): Token[] {
  const out: Token[] = []
  let   inLink       = 0

  for (const child of children) {
    if (options.skipInsideLinks) {
      if (child.type === 'link_open')  inLink++
      if (child.type === 'link_close') inLink--
    }
    if (child.type !== 'text' || inLink > 0) {
      out.push(child)
      continue
    }
    const replaced = splitTextToken(child, pattern, TokenCtor, m => replace(m, child))
    if (Array.isArray(replaced)) out.push(...replaced)
    else                         out.push(replaced)
  }

  return out
}

function makeText(content: string, level: number, TokenCtor: TokenCtor): Token {
  const t   = new TokenCtor('text', '', 0)
  t.content = content
  t.level   = level
  return t
}
