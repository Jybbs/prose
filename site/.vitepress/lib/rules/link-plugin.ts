import type MarkdownIt from 'markdown-it'

import { walkBodyInlines } from '../markdown/walk'

export function ruleLinkPlugin(
  validRuleSlugs : Set<string>,
  primitiveNames : ReadonlyMap<string, string>
): (md: MarkdownIt) => void {
  return function plugin(md: MarkdownIt): void {
    md.inline.ruler.before('link', 'doc-link', (state, silent) => {
      if (state.src.slice(state.pos, state.pos + 2) !== '[[') return false
      const end = state.src.indexOf(']]', state.pos + 2)
      if (end === -1) return false
      const slug = state.src.slice(state.pos + 2, end)
      if (!/^[a-z][a-z0-9-]*$/.test(slug)) return false

      let kind: 'primitive' | 'rule'
      if (validRuleSlugs.has(slug))      kind = 'rule'
      else if (primitiveNames.has(slug)) kind = 'primitive'
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

    md.core.ruler.after('inline', 'doc-link-code', state => {
      walkBodyInlines(state, (_block, children) => {
        for (const child of children) {
          if (child.type !== 'code_inline')       continue
          if (!validRuleSlugs.has(child.content)) continue
          child.type = 'doc_link'
          child.tag  = ''
          child.meta = { kind: 'rule' }
        }
      })
    })

    md.renderer.rules.doc_link = (tokens, idx) => {
      const slug = tokens[idx].content
      if (tokens[idx].meta?.kind === 'rule') {
        return `<InlineRuleLink slug="${slug}" />`
      }
      const display = primitiveNames.get(slug)!
      return (
        `<a class="body-link" href="/primitives/${slug}">`
        + `<strong><code>${display}</code></strong></a>`
      )
    }
  }
}
