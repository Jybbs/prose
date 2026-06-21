import type MarkdownIt from 'markdown-it'

import { replaceTextTokens } from '../markdown/token-split'
import { walkBodyInlines }   from '../markdown/walk'

export function glossaryPlugin(phraseToSlug: ReadonlyMap<string, string>): (md: MarkdownIt) => void {
  if (phraseToSlug.size === 0) {
    throw new Error('glossaryPlugin received an empty phrase map')
  }

  const phrases = [...phraseToSlug.keys()].toSorted((a, b) => b.length - a.length)
  const pattern = new RegExp(
    `(?<![A-Za-z0-9_-])(${phrases.map(p => RegExp.escape(p)).join('|')})(?![A-Za-z0-9_-])`,
    'g'
  )

  return function plugin(md: MarkdownIt): void {
    md.core.ruler.after('inline', 'glossary-decorate', state => {
      const seen: Set<string> = (state.env.seenGlossarySlugs ??= new Set())
      walkBodyInlines(state, (block, children) => {
        block.children = replaceTextTokens(children, state.Token, pattern, (match, child) => {
          const phrase = match[1]
          const slug   = phraseToSlug.get(phrase)!
          if (seen.has(slug)) return null
          seen.add(slug)
          const term   = new state.Token('glossary_term', '', 0)
          term.content = phrase
          term.meta    = { slug }
          term.level   = child.level
          return [term]
        }, { skipInsideLinks: true })
      })
    })

    md.renderer.rules.glossary_term = (tokens, idx) => {
      const t       = tokens[idx]
      const slug    = md.utils.escapeHtml(t.meta?.slug as string)
      const display = md.utils.escapeHtml(t.content)
      return `<GlossaryTerm slug="${slug}">${display}</GlossaryTerm>`
    }
  }
}
