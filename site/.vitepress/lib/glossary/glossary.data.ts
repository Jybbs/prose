import { defineLoader } from 'vitepress'

import { glossary, type GlossaryEntry }       from './entries'
import { entryHref }                          from './hrefs'
import { getRenderer, renderPlainInlineHtml } from '../markdown/renderer'
import { inlineNodes, type InlineNode }       from '../markdown/inline-nodes'
import { discoverRuleIndex }                  from '../rules/discovery'
import { rulesDir }                           from '../shared/paths'
import type { GlossaryFamily }                from '../shared/registries'

const ruleIndex = discoverRuleIndex(rulesDir(import.meta.url))

export interface RenderedGlossaryEntry {
  aliases         : readonly string[]
  definitionHtml  : string
  definitionNodes : InlineNode[]
  families        : readonly GlossaryFamily[]
  href           ?: string
  initial         : string
  primaryFamily   : GlossaryFamily
  slug            : string
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
      const families = entryFamilies(entry, slug)
      entries[slug] = {
        aliases         : entry.aliases ?? [],
        definitionHtml  : renderPlainInlineHtml(md, entry.definition),
        definitionNodes : inlineNodes(md, entry.definition),
        families        : families,
        href            : entryHref(slug, entry, ruleIndex),
        initial         : firstLetter(slug),
        primaryFamily   : families[0],
        slug            : slug
      }
    }

    return { entries }
  }
})

function entryFamilies(entry: GlossaryEntry, slug: string): readonly GlossaryFamily[] {
  const declared = entry.families ?? []
  if (!entry.rule) {
    if (declared.length === 0) throw new Error(`Glossary entry "${slug}" declares no family`)
    return declared
  }
  const rule = ruleIndex.get(entry.rule)
  if (!rule) throw new Error(`Glossary entry "${slug}" names unknown rule "${entry.rule}"`)
  return [rule.family as GlossaryFamily, ...declared]
}

function firstLetter(slug: string): string {
  return slug.match(/[a-z]/i)?.[0].toUpperCase() ?? '#'
}
