import { visitParents }    from 'unist-util-visit-parents'
import type { Properties } from 'hast'
import type { Data, Link, Nodes, Parent, PhrasingContent, Root } from 'mdast'

// mdast-util-to-hast reads `hName` and `hProperties` off any node's `data`, so
// a custom element reaches hast without a handler. The type stays off
// the mdast unions, so body-link, which visits `link` nodes, leaves these be.
export function mdastElement(
  hName      : string,
  properties : Properties,
  children   : PhrasingContent[]
): PhrasingContent {
  const node = { type: 'proseElement', children, data: { hName, hProperties: properties } }
  return node as unknown as PhrasingContent
}

// A native link, so `body-link` and to-hast both treat it as an anchor.
export function mdastLink(url: string, children: PhrasingContent[]): Link {
  return { type: 'link', url, title: null, children }
}

export function mdastSpan(className: string, children: PhrasingContent[]): PhrasingContent {
  return mdastElement('span', { className: [className] }, children)
}

export const mdastText = (value: string): PhrasingContent[] => [{ type: 'text', value }]

// hast `className` is a string or an array, so coerce before appending.
export function pushClassName(node: { data?: Data }, className: string): void {
  const data       = (node.data ??= {})
  const properties = (data.hProperties ??= {})
  const existing   = properties.className
  const list       = Array.isArray(existing) ? existing : typeof existing === 'string' ? [existing] : []
  properties.className = [...list, className]
}

const withinHeading = (ancestors: Array<{ type: string }>): boolean =>
  ancestors.some(ancestor => ancestor.type === 'heading')

// Visits every node of `type` that no heading encloses.
export function visitOutsideHeadings<T extends Nodes['type']>(
  tree    : Root,
  type    : T,
  visitor : (node: Extract<Nodes, { type: T }>, ancestors: Parent[]) => void
): void {
  visitParents(tree, type as Nodes['type'], (node, ancestors) => {
    if (withinHeading(ancestors)) return
    visitor(node as Extract<Nodes, { type: T }>, ancestors as Parent[])
  })
}

// The lookarounds keep hyphenated and snake_case compounds literal.
export const wordBounded = (source: string): RegExp =>
  new RegExp(String.raw`(?<![\w-])(${source})(?![\w-])`, 'g')
