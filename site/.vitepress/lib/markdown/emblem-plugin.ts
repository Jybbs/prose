import type MarkdownIt from 'markdown-it'

import type { Emblem }       from '../shared/emblems'
import { nibBadgeHtml }      from '../shared/nib'
import { replaceTextTokens } from './token-split'
import { walkBodyInlines }   from './walk'

// Sets each emoji the map names, wherever it stands in body text, as its
// emblem inside the running-text chip at the body's 16 px, labeled with its
// motif since no text beside it names the badge.
export function emblemPlugin(emblems: ReadonlyMap<string, Emblem>): (md: MarkdownIt) => void {
  const pattern = new RegExp([...emblems.keys()].map(emoji => RegExp.escape(emoji)).join('|'), 'gu')
  return function plugin(md: MarkdownIt): void {
    md.core.ruler.after('inline', 'emblem', state => {
      walkBodyInlines(state, (block, children) => {
        block.children = replaceTextTokens(children, state.Token, pattern, (match, child) => {
          const emblem = emblems.get(match[0])!
          const family = emblem.family ? ` data-family="${emblem.family}"` : ''
          const chip   = new state.Token('html_inline', '', 0)
          chip.content = `<span class="nib-chip"${family}>`
                       + `${nibBadgeHtml(emblem, 16, emblem.motif.label)}</span>`
          chip.level   = child.level
          return [chip]
        })
      })
    })
  }
}
