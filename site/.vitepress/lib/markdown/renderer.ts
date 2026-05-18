import { createMarkdownRenderer, type MarkdownRenderer } from 'vitepress'

import { siteDir } from '../shared/paths'

let cachedRenderer: Promise<MarkdownRenderer> | null = null

export function getRenderer(): Promise<MarkdownRenderer> {
  if (cachedRenderer === null) cachedRenderer = createMarkdownRenderer(siteDir(import.meta.url))
  return cachedRenderer
}

type HtmlKey<K extends string> = `${K}Html`

type Rendered<T, K extends string & keyof T> =
  Omit<T, K> & { [P in HtmlKey<K>]: T[K] extends readonly string[] ? string[] : string }

export function renderInlineField<T extends object, K extends string & keyof T>(
  md    : MarkdownRenderer,
  items : readonly T[],
  field : K
): Array<Rendered<T, K>> {
  return items.map(item => {
    const value     = item[field]
    const rendered  = Array.isArray(value)
      ? (value as readonly string[]).map(s => md.renderInline(s))
      : md.renderInline(value as string)
    const { [field]: _, ...rest } = item
    return { ...rest, [`${field}Html`]: rendered } as Rendered<T, K>
  })
}
