import { defineLoader } from 'vitepress'

import { glossary, type GlossaryEntry } from '../lib/glossary/entries'
import { getRenderer }                  from '../lib/markdown/renderer'
import { discoverRuleSlugs }            from '../lib/rules/discovery'
import { rulesDir }                     from '../lib/shared/paths'
import type { GlossaryFamily }          from '../lib/shared/registries'

const ruleHrefs = new Map(discoverRuleSlugs(rulesDir(import.meta.url)).map(r => [r.slug, r.href]))

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
        href           : entryHref(slug, entry),
        initial        : firstLetter(slug),
        primaryFamily  : entry.families[0],
        slug
      }
    }

    return { entries }
  }
})

function entryHref(slug: string, entry: GlossaryEntry): string | undefined {
  if (entry.rule !== undefined) {
    const href = ruleHrefs.get(entry.rule)
    if (!href) throw new Error(`Glossary "${slug}" names unknown rule "${entry.rule}"`)
    return href
  }
  if (entry.href?.startsWith('/rules/')) {
    throw new Error(`Glossary "${slug}" hand-writes a rule URL, use the rule field instead`)
  }
  return entry.href
}

function firstLetter(slug: string): string {
  return slug.match(/[a-z]/i)?.[0].toUpperCase() ?? '#'
}
