import type { Properties }                  from 'hast'
import type { Data, Link, PhrasingContent } from 'mdast'

export const mdastText = (value: string): PhrasingContent[] => [{ type: 'text', value }]

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

export function mdastSpan(className: string, children: PhrasingContent[]): PhrasingContent {
  return mdastElement('span', { className: [className] }, children)
}

// A native link, so `body-link` and to-hast both treat it as an anchor. Extra
// hast attributes, a class or a `data-*`, go in `data.hProperties`.
export function mdastLink(url: string, properties: Properties, children: PhrasingContent[]): Link {
  return { type: 'link', url, title: null, children, data: { hProperties: properties } }
}

// hast `className` is a string or an array, so coerce before appending.
export function pushClassName(node: { data?: Data }, className: string): void {
  const data       = (node.data ??= {})
  const properties = (data.hProperties ??= {})
  const existing   = properties.className
  const list       = Array.isArray(existing) ? existing : typeof existing === 'string' ? [existing] : []
  properties.className = [...list, className]
}

export const withinHeading = (ancestors: Array<{ type: string }>): boolean =>
  ancestors.some(ancestor => ancestor.type === 'heading')

// The lookarounds keep hyphenated and snake_case compounds literal.
export const wordBounded = (source: string): RegExp =>
  new RegExp(String.raw`(?<![\w-])(${source})(?![\w-])`, 'g')
