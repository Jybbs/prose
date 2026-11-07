import { createMarkdownRenderer, type MarkdownRenderer } from 'vitepress'

import { codeUnitPlugin }                         from './code-operators'
import { inlineNodes, type InlineNode }           from './inline-nodes'
import { SHIKI_COLOR_REPLACEMENTS, SHIKI_THEMES } from '../shared/constants'
import { siteDir }                                from '../shared/paths'
import { plainTermsEnv }                          from './inert-env'
import { TokenClasses }                           from './token-classes'

// Maps each field `K` to `${K}${S}`, carrying one `V` where the source field
// is a scalar and a `V[]` where it is a list of strings.
type Suffixed<T, K extends string & keyof T, S extends string, V> =
  Omit<T, K> & { [P in `${K}${S}`]: T[K] extends readonly string[] ? V[] : V }

type Rendered<T, K extends string & keyof T> = Suffixed<T, K, 'Html', string>

type Walked<T, K extends string & keyof T> = Suffixed<T, K, 'Nodes', InlineNode[]>

export async function getRenderer(): Promise<MarkdownRenderer> {
  const md = await createMarkdownRenderer(siteDir(import.meta.url), {
    codeTransformers  : [(await TokenClasses.read()).transformer()],
    colorReplacements : SHIKI_COLOR_REPLACEMENTS,
    theme             : SHIKI_THEMES
  })
  md.use(codeUnitPlugin)
  return md
}

// Prose a component renders as live markup walks to a node tree, whereas the
// `*Html` renderers stay for the strings a popup or a plain-terms caption
// consumes, where a mounted component cannot go.
export function inlineNodeField<T extends object, K extends string & keyof T>(
  md    : MarkdownRenderer,
  items : readonly T[],
  field : K
): Array<Walked<T, K>> {
  return items.map(item => {
    const value  = item[field]
    const walked = Array.isArray(value)
      ? (value as readonly string[]).map(entry => inlineNodes(md, entry))
      : inlineNodes(md, value as string)
    return suffixed<Walked<T, K>>(item, field, 'Nodes', walked)
  })
}

export function renderFencedField<T extends { language: string }, K extends string & keyof T>(
  md    : MarkdownRenderer,
  items : readonly T[],
  field : K
): Promise<Array<Rendered<T, K>>> {
  return Promise.all(items.map(async item => suffixed<Rendered<T, K>>(
    item, field, 'Html', await renderFencedHtml(md, item[field] as string, item.language))))
}

export function renderFencedHtml(
  md       : MarkdownRenderer,
  code     : string,
  language : string,
  meta     : string = ''
): Promise<string> {
  return md.renderAsync(`\`\`\`${language}${meta ? ` ${meta}` : ''}\n${code}\n\`\`\``)
}

// Caption text renders inside cover-linked cards and hover popups, where a
// glossary anchor cannot receive its own click, so terms flatten to text.
export function renderPlainInlineHtml(md: MarkdownRenderer, src: string): string {
  return md.renderInline(src, plainTermsEnv())
}

function suffixed<R>(item: object, field: string, suffix: string, mapped: unknown): R {
  const { [field]: _, ...rest } = item as Record<string, unknown>
  return { ...rest, [`${field}${suffix}`]: mapped } as R
}
