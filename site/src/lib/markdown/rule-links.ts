import { findAndReplace }             from 'mdast-util-find-and-replace'
import type { PhrasingContent, Root } from 'mdast'
import { visitParents }               from 'unist-util-visit-parents'

import type { DocsVocab, PrimitiveRef, RuleRef }  from '../content/docs-vocab'
import { mdastElement, mdastLink, withinHeading } from './mdast-node'

const SLUG      = /^[a-z][a-z0-9-]*$/
const WIKI_LINK = /\[\[([^\]]+)\]\]/g

const ruleNode = (ref: RuleRef, slug: string): PhrasingContent =>
  mdastElement('a', { className: ['rule-chip'], 'data-caption': ref.caption, href: ref.href }, [
    { type: 'text', value: slug }
  ])

const primitiveNode = (ref: PrimitiveRef): PhrasingContent =>
  mdastLink(ref.href, {}, [{ type: 'strong', children: [{ type: 'inlineCode', value: ref.title }] }])

// Two-phase rule and primitive linking. Phase one rewrites [[slug]] in body or
// heading text, a rule to a rule-chip component and a primitive to a plain
// anchor, throwing on a well-formed but unknown slug. Phase two promotes a bare
// backtick slug whose value is a known rule, outside headings.
export function remarkRuleLinks(vocab: DocsVocab) {
  return (tree: Root): void => {
    findAndReplace(tree, [WIKI_LINK, (_match, slug: string) => {
      if (!SLUG.test(slug)) return false
      const rule = vocab.rules.get(slug)
      if (rule) return ruleNode(rule, slug)
      const primitive = vocab.primitives.get(slug)
      if (primitive) return primitiveNode(primitive)
      throw new Error(`Unknown slug "${slug}" referenced by [[${slug}]]`)
    }])

    const promotions: Array<{ children: PhrasingContent[], index: number, node: PhrasingContent }> = []
    visitParents(tree, 'inlineCode', (node, ancestors) => {
      if (withinHeading(ancestors)) return
      const rule = vocab.rules.get(node.value)
      if (!rule) return
      const children = ancestors.at(-1)!.children as unknown as PhrasingContent[]
      promotions.push({ children, index: children.indexOf(node), node: ruleNode(rule, node.value) })
    })
    for (const { children, index, node } of promotions) children[index] = node
  }
}
