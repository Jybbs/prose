import { createMarkdownRenderer, defineLoader } from 'vitepress'

import { glossary } from '../lib/glossary'
import { siteDir }  from '../lib/paths'

const root = siteDir(import.meta.url)

export interface RenderedGlossaryEntry {
  definitionHtml : string
  href          ?: string
}

export interface GlossaryData {
  entries     : Record<string, RenderedGlossaryEntry>
  phraseToSlug: Record<string, string>
}

declare const data: GlossaryData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<GlossaryData> {
    const md           = await createMarkdownRenderer(root)
    const entries      : Record<string, RenderedGlossaryEntry> = {}
    const phraseToSlug : Record<string, string>                = {}

    for (const [slug, entry] of Object.entries(glossary)) {
      entries[slug] = {
        definitionHtml: md.renderInline(entry.definition),
        href          : entry.href
      }
      registerPhrase(phraseToSlug, slug, slug)
      for (const alias of entry.aliases ?? []) {
        registerPhrase(phraseToSlug, alias, slug)
      }
    }

    return { entries, phraseToSlug }
  }
})

function registerPhrase(map: Record<string, string>, phrase: string, slug: string): void {
  const existing = map[phrase]
  if (existing !== undefined && existing !== slug) {
    throw new Error(
      `Glossary phrase "${phrase}" registered against both "${existing}" and "${slug}"`
    )
  }
  map[phrase] = slug
}
