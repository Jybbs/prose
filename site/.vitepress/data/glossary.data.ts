import { defineLoader } from 'vitepress'

import { glossary }     from '../lib/glossary'
import { siteDir }      from '../lib/paths'
import { getRenderer }  from '../lib/render-markdown'

const root = siteDir(import.meta.url)

export interface RenderedGlossaryEntry {
  definitionHtml : string
  href          ?: string
}

export interface GlossaryData {
  entries: Record<string, RenderedGlossaryEntry>
}

declare const data: GlossaryData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<GlossaryData> {
    const md      = await getRenderer(root)
    const entries : Record<string, RenderedGlossaryEntry> = {}

    for (const [slug, entry] of Object.entries(glossary)) {
      entries[slug] = {
        definitionHtml: md.renderInline(entry.definition),
        href          : entry.href
      }
    }

    return { entries }
  }
})
