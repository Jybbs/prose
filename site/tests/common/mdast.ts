import { toHtml }       from 'hast-util-to-html'
import { fromMarkdown } from 'mdast-util-from-markdown'
import { toHast }       from 'mdast-util-to-hast'
import type { Root }    from 'mdast'

export type Transform = (tree: Root) => void

const applyTransform = (transform: Transform, markdown: string): Root => {
  const tree = fromMarkdown(markdown)
  transform(tree)
  return tree
}

export const renderTransform = (transform: Transform, markdown: string): string =>
  toHtml(toHast(applyTransform(transform, markdown), { allowDangerousHtml: true })!)
