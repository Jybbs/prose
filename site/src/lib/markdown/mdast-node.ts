import type { Properties }            from 'hast'
import type { Link, PhrasingContent } from 'mdast'

interface HastData {
  hName       ?: string
  hProperties ?: Properties
}

// A single text node wrapped for the phrasing-content slots the builders take.
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

// A span wrapper needs no component, so the word-mark and its kin take this.
export function mdastSpan(className: string, children: PhrasingContent[]): PhrasingContent {
  return mdastElement('span', { className: [className] }, children)
}

// A native link, so `body-link` and to-hast both treat it as an anchor. Extra
// hast attributes, a class or a `data-*`, go in `data.hProperties`.
export function mdastLink(url: string, properties: Properties, children: PhrasingContent[]): Link {
  return { type: 'link', url, title: null, children, data: { hProperties: properties } }
}

// Appends a class to a node's hast properties, coercing the string-or-array
// `className` shape so an existing list survives.
export function pushClassName(node: { data?: HastData }, className: string): void {
  const data       = (node.data ??= {})
  const properties = (data.hProperties ??= {})
  const existing   = properties.className
  const list       = Array.isArray(existing) ? existing : typeof existing === 'string' ? [existing] : []
  properties.className = [...list, className]
}

// True when any ancestor is a heading, so the visitors skip heading-nested nodes.
export const withinHeading = (ancestors: Array<{ type: string }>): boolean =>
  ancestors.some(ancestor => ancestor.type === 'heading')
