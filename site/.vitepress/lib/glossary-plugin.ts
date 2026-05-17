import type MarkdownIt from 'markdown-it'
import type Token       from 'markdown-it/lib/token.mjs'

const ESCAPE_REGEX = /[.*+?^${}()|[\]\\]/g

export function glossaryPlugin(phraseToSlug: ReadonlyMap<string, string>) {
  if (phraseToSlug.size === 0) {
    return (_md: MarkdownIt): void => {}
  }

  const phrases  = [...phraseToSlug.keys()].sort((a, b) => b.length - a.length)
  const pattern  = new RegExp(
    `(?<![A-Za-z0-9_-])(${phrases.map(p => p.replace(ESCAPE_REGEX, '\\$&')).join('|')})(?![A-Za-z0-9_-])`,
    'g'
  )

  return function plugin(md: MarkdownIt) {
    md.core.ruler.after('inline', 'glossary-decorate', state => {
      const seen: Set<string> = (state.env.seenGlossarySlugs ??= new Set())

      for (let i = 0; i < state.tokens.length; i++) {
        const block = state.tokens[i]
        if (block.type !== 'inline' || !block.children) continue
        const prev = state.tokens[i - 1]
        if (prev?.type.startsWith('heading_')) continue

        block.children = decorateChildren(block.children, pattern, phraseToSlug, seen, state.Token)
      }
    })

    md.renderer.rules.glossary_term = (tokens, idx) => {
      const t       = tokens[idx]
      const slug    = t.meta?.slug as string
      const display = t.content
      return `<GlossaryTerm slug="${md.utils.escapeHtml(slug)}">${md.utils.escapeHtml(display)}</GlossaryTerm>`
    }
  }
}

function decorateChildren(
  children    : Token[],
  pattern     : RegExp,
  phraseToSlug: ReadonlyMap<string, string>,
  seen        : Set<string>,
  TokenCtor   : new (type: string, tag: string, nesting: number) => Token
): Token[] {
  const out: Token[] = []
  let   inLink       = 0

  for (const child of children) {
    if (child.type === 'link_open')  inLink++
    if (child.type === 'link_close') inLink--

    if (child.type !== 'text' || inLink > 0) {
      out.push(child)
      continue
    }

    const replaced = decorateText(child.content, pattern, phraseToSlug, seen, TokenCtor, child.level)
    if (replaced.length === 1 && replaced[0].type === 'text' && replaced[0].content === child.content) {
      out.push(child)
    }
    else {
      out.push(...replaced)
    }
  }

  return out
}

function decorateText(
  text        : string,
  pattern     : RegExp,
  phraseToSlug: ReadonlyMap<string, string>,
  seen        : Set<string>,
  TokenCtor   : new (type: string, tag: string, nesting: number) => Token,
  level       : number
): Token[] {
  pattern.lastIndex = 0
  const out: Token[] = []
  let   cursor       = 0
  let   match: RegExpExecArray | null

  while ((match = pattern.exec(text)) !== null) {
    const phrase = match[1]
    const slug   = phraseToSlug.get(phrase)!
    if (seen.has(slug)) continue

    if (match.index > cursor) {
      const leading     = new TokenCtor('text', '', 0)
      leading.content   = text.slice(cursor, match.index)
      leading.level     = level
      out.push(leading)
    }

    const term     = new TokenCtor('glossary_term', '', 0)
    term.content   = phrase
    term.meta      = { slug }
    term.level     = level
    out.push(term)
    seen.add(slug)
    cursor = match.index + phrase.length
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
