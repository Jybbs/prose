import { createMarkdownRenderer, type MarkdownRenderer } from 'vitepress'

import { siteDir }  from '../shared/paths'
import { inertEnv } from './inert-env'

let cachedRenderer: Promise<MarkdownRenderer> | null = null

export function getRenderer(): Promise<MarkdownRenderer> {
  if (cachedRenderer === null) cachedRenderer = createMarkdownRenderer(siteDir(import.meta.url))
  return cachedRenderer
}

type HtmlKey<K extends string> = `${K}Html`

type Rendered<T, K extends string & keyof T> =
  Omit<T, K> & { [P in HtmlKey<K>]: T[K] extends readonly string[] ? string[] : string }

export function renderBlockHtml(md: MarkdownRenderer, src: string): Promise<string> {
  return md.renderAsync(src, inertEnv())
}

export function renderFencedField<T extends { language: string }, K extends string & keyof T>(
  md    : MarkdownRenderer,
  items : readonly T[],
  field : K
): Promise<Array<Rendered<T, K>>> {
  return Promise.all(items.map(async item => {
    const rendered = await renderFencedHtml(md, item[field] as string, item.language)
    const { [field]: _, ...rest } = item
    return { ...rest, [`${field}Html`]: rendered } as Rendered<T, K>
  }))
}

export function renderFencedHtml(
  md       : MarkdownRenderer,
  code     : string,
  language : string,
  meta     : string = ''
): Promise<string> {
  return md.renderAsync(`\`\`\`${language}${meta ? ` ${meta}` : ''}\n${code}\n\`\`\``)
}

export function renderInlineField<T extends object, K extends string & keyof T>(
  md    : MarkdownRenderer,
  items : readonly T[],
  field : K
): Array<Rendered<T, K>> {
  return items.map(item => {
    const value     = item[field]
    const rendered  = Array.isArray(value)
      ? (value as readonly string[]).map(s => renderInlineHtml(md, s))
      : renderInlineHtml(md, value as string)
    const { [field]: _, ...rest } = item
    return { ...rest, [`${field}Html`]: rendered } as Rendered<T, K>
  })
}

export function renderInlineHtml(md: MarkdownRenderer, src: string): string {
  return md.renderInline(src, inertEnv())
}
