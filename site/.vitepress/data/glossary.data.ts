import { createMarkdownRenderer, defineLoader } from 'vitepress'

import { glossary } from '../lib/glossary'
import { siteDir }  from '../lib/paths'

export interface RenderedGlossaryEntry {
  definitionHtml : string
  href          ?: string
}

declare const data: Record<string, RenderedGlossaryEntry>
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<Record<string, RenderedGlossaryEntry>> {
    const md  = await createMarkdownRenderer(siteDir(import.meta.url))
    const out: Record<string, RenderedGlossaryEntry> = {}
    for (const [term, entry] of Object.entries(glossary)) {
      out[term] = {
        definitionHtml: md.renderInline(entry.definition),
        href          : entry.href
      }
    }
    return out
  }
})
