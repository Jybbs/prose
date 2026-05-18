import type MarkdownIt from 'markdown-it'

import { PRIMITIVES } from './registries'

export function ruleLinkPlugin(validRuleSlugs: Set<string>) {
  return function plugin(md: MarkdownIt) {
    md.inline.ruler.before('link', 'doc-link', (state, silent) => {
      if (state.src.slice(state.pos, state.pos + 2) !== '[[') return false
      const end = state.src.indexOf(']]', state.pos + 2)
      if (end === -1) return false
      const slug = state.src.slice(state.pos + 2, end)
      if (!/^[a-z][a-z0-9-]*$/.test(slug)) return false

      let kind: 'primitive' | 'rule'
      if (validRuleSlugs.has(slug)) kind = 'rule'
      else if (slug in PRIMITIVES)  kind = 'primitive'
      else {
        throw new Error(`Unknown slug "${slug}" referenced by [[${slug}]]`)
      }

      if (!silent) {
        const token   = state.push('doc_link', '', 0)
        token.content = slug
        token.meta    = { kind }
      }
      state.pos = end + 2
      return true
    })

    md.renderer.rules.doc_link = (tokens, idx) => {
      const slug    = tokens[idx].content
      const kind    = tokens[idx].meta?.kind === 'primitive' ? 'primitives' : 'rules'
      const display = kind === 'primitives' ? PRIMITIVES[slug as keyof typeof PRIMITIVES] : slug
      return `<a href="/${kind}/${slug}"><strong><code>${display}</code></strong></a>`
    }
  }
}
