import { defineLoader } from 'vitepress'

import { glossary }                                  from '../lib/glossary/entries'
import { getRenderer }                               from '../lib/markdown/renderer'
import { GLOSSARY_FAMILY_META, type GlossaryFamily } from '../lib/shared/registries'

export interface RenderedGlossaryEntry {
  aliases        : readonly string[]
  definitionHtml : string
  family         : GlossaryFamily
  familyBadge    : string
  familyLabel    : string
  href          ?: string
  initial        : string
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
        family         : entry.family,
        familyBadge    : GLOSSARY_FAMILY_META[entry.family].badge,
        familyLabel    : GLOSSARY_FAMILY_META[entry.family].label,
        href           : entry.href,
        initial        : firstLetter(slug),
        slug
      }
    }

    return { entries }
  }
})

function firstLetter(slug: string): string {
  return slug.match(/[a-z]/i)?.[0].toUpperCase() ?? '#'
}
