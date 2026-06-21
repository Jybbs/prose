interface SplittableToken {
  content: string
  level  : number
  type   : string
}

type TokenCtor<T> = new (type: string, tag: string, nesting: 0 | 1 | -1) => T

function splitTextToken<T extends SplittableToken>(
  child    : T,
  pattern  : RegExp,
  TokenCtor: TokenCtor<T>,
  replace  : (match: RegExpExecArray) => T[] | null
): T | T[] {
  const text = child.content
  pattern.lastIndex = 0
  const out: T[] = []
  let   cursor   = 0
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

export function replaceTextTokens<T extends SplittableToken>(
  children : T[],
  TokenCtor: TokenCtor<T>,
  pattern  : RegExp,
  replace  : (match: RegExpExecArray, child: T) => T[] | null,
  options  : { skipInsideLinks?: boolean } = {}
): T[] {
  const out: T[] = []
  let   inLink   = 0

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

function makeText<T extends SplittableToken>(
  content  : string,
  level    : number,
  TokenCtor: TokenCtor<T>
): T {
  const t   = new TokenCtor('text', '', 0)
  t.content = content
  t.level   = level
  return t
}
