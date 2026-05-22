import { defineLoader } from 'vitepress'

import { glossary }    from '../lib/glossary/glossary'
import { getRenderer } from '../lib/markdown/renderer'

export interface RenderedGlossaryEntry {
  aliases        : readonly string[]
  definitionHtml : string
  href          ?: string
  initial        : string
  slug           : string
}

export interface GlossaryData {
  entries: Record<string, RenderedGlossaryEntry>
}

declare const data: GlossaryData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<GlossaryData> {
    const md      = await getRenderer()
    const entries : Record<string, RenderedGlossaryEntry> = {}

    for (const [slug, entry] of Object.entries(glossary)) {
      entries[slug] = {
        aliases        : entry.aliases ?? [],
        definitionHtml : md.renderInline(entry.definition),
        href           : entry.href,
        initial        : firstLetter(slug),
        slug
      }
    }

    return { entries }
  }
})

function firstLetter(slug: string): string {
  return Iterator.from(slug).find(ch => /[A-Za-z]/.test(ch))?.toUpperCase() ?? '#'
}
