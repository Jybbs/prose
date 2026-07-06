import type MarkdownIt from 'markdown-it'

import { isInert }                  from '../markdown/inert-env'
import { walkBodyInlines }          from '../markdown/walk'
import type { DiscoveredPrimitive } from '../primitives/discovery'
import { BODY_LINK_CLASSES }        from '../shared/constants'
import type { DiscoveredRule }      from './discovery'

export function ruleLinkPlugin(
  rules      : ReadonlyMap<string, Pick<DiscoveredRule, 'family' | 'href'>>,
  primitives : ReadonlyMap<string, Pick<DiscoveredPrimitive, 'name'>>
): (md: MarkdownIt) => void {
  return function plugin(md: MarkdownIt): void {
    md.inline.ruler.before('link', 'doc-link', (state, silent) => {
      if (state.src.slice(state.pos, state.pos + 2) !== '[[') return false
      const end = state.src.indexOf(']]', state.pos + 2)
      if (end === -1) return false
      const slug = state.src.slice(state.pos + 2, end)
      if (!/^[A-Za-z][A-Za-z0-9-]*$/.test(slug)) return false

      let kind: 'primitive' | 'rule'
      if (rules.has(slug))           kind = 'rule'
      else if (primitives.has(slug)) kind = 'primitive'
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
          if (child.type !== 'code_inline') continue
          if (!rules.has(child.content))    continue
          child.type = 'doc_link'
          child.tag  = ''
          child.meta = { kind: 'rule' }
        }
      })
    })

    md.renderer.rules.doc_link = (tokens, idx, _options, env) => {
      const slug = tokens[idx].content
      if (tokens[idx].meta?.kind === 'rule') {
        if (!isInert(env)) return `<InlineRuleLink slug="${slug}" />`
        const rule = rules.get(slug)!
        return `<a class="rule-link" data-family="${rule.family}" href="${rule.href}">${slug}</a>`
      }
      const display = primitives.get(slug)!.name
      return (
        `<a class="${BODY_LINK_CLASSES}" href="/primitives/${slug}">`
        + `<strong><code>${display}</code></strong></a>`
      )
    }
  }
}
