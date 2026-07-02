import escapeStringRegexp from 'escape-string-regexp'
import type { Root }      from 'mdast'
import { findAndReplace } from 'mdast-util-find-and-replace'

import type { DocsVocab, GlossaryRef }          from '../content/docs-vocab'
import { mdastElement, mdastText, wordBounded } from './mdast-node'

const glossaryNode = (ref: GlossaryRef, phrase: string) => {
  const text  = mdastText(phrase)
  const props = { className: ['glossary-term'], 'data-definition': ref.definition }
  return ref.href
    ? mdastElement('a', { ...props, href: ref.href }, text)
    : mdastElement('span', props, text)
}

// Auto-links the first occurrence of each glossary entry per page, longest
// phrase first so a phrase never shadows one it prefixes, wrapping the matched
// casing in a glossary-term component. `ignore` leaves headings and existing
// links untouched, and find-and-replace never enters code.
export function remarkGlossary(vocab: DocsVocab) {
  const phrases = [...vocab.glossary.keys()].sort((a, b) => b.length - a.length)
  if (phrases.length === 0) return () => {}
  const pattern = wordBounded(phrases.map(escapeStringRegexp).join('|'))
  return (tree: Root): void => {
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
