import { defineLoader } from 'vitepress'

import { glossary }            from '../lib/glossary/entries'
import { getRenderer }         from '../lib/markdown/renderer'
import type { GlossaryFamily } from '../lib/shared/registries'

export interface RenderedGlossaryEntry {
  aliases        : readonly string[]
  definitionHtml : string
  families       : readonly GlossaryFamily[]
  href          ?: string
  initial        : string
  primaryFamily  : GlossaryFamily
  slug           : string
}

interface GlossaryData {
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
        families       : entry.families,
        href           : entry.href,
        initial        : firstLetter(slug),
        primaryFamily  : entry.families[0],
        slug
      }
    }

    return { entries }
  }
})

function firstLetter(slug: string): string {
  return slug.match(/[a-z]/i)?.[0].toUpperCase() ?? '#'
}
