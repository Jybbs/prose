import escapeStringRegexp from 'escape-string-regexp'
import type { Root }      from 'mdast'
import { findAndReplace } from 'mdast-util-find-and-replace'

import type { DocsVocab, GlossaryRef } from '../content/docs-vocab'
import { mdastElement }                from './mdast-node'

const glossaryNode = (ref: GlossaryRef, phrase: string) => {
  const text = [{ type: 'text' as const, value: phrase }]
  return ref.href
    ? mdastElement('a', { className: ['glossary-term'], 'data-definition': ref.definition, href: ref.href }, text)
    : mdastElement('span', { className: ['glossary-term'], 'data-definition': ref.definition }, text)
}

// Auto-links the first occurrence of each glossary phrase per page, longest
// phrase first so a phrase never shadows one it prefixes, wrapping the matched
// casing in a glossary-term component. `ignore` leaves existing links, code, and
// headings untouched.
export function remarkGlossary(vocab: DocsVocab) {
  const phrases = [...vocab.glossary.keys()].sort((a, b) => b.length - a.length)
  const pattern = new RegExp(
    `(?<![A-Za-z0-9_-])(${phrases.map(escapeStringRegexp).join('|')})(?![A-Za-z0-9_-])`,
    'g'
  )
  return (tree: Root): void => {
    if (phrases.length === 0) return
    const seen = new Set<string>()
    findAndReplace(
      tree,
      [pattern, (_match, phrase: string) => {
        const ref = vocab.glossary.get(phrase)!
        if (seen.has(ref.slug)) return false
        seen.add(ref.slug)
        return glossaryNode(ref, phrase)
      }],
      { ignore: ['heading', 'link', 'linkReference'] }
    )
  }
}
