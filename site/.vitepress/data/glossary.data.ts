import { defineLoader } from 'vitepress'

import { glossary }                       from '../lib/glossary/entries'
import { entryHref }                      from '../lib/glossary/hrefs'
import { getRenderer, renderInlineHtml }  from '../lib/markdown/renderer'
import { discoverRuleIndex }              from '../lib/rules/discovery'
import { rulesDir }                       from '../lib/shared/paths'
import type { GlossaryFamily }            from '../lib/shared/registries'

const ruleIndex = discoverRuleIndex(rulesDir(import.meta.url))

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
        definitionHtml : renderInlineHtml(md, entry.definition),
        families       : entry.families,
        href           : entryHref(slug, entry, ruleIndex),
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
