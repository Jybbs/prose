import { getCollection } from 'astro:content'

import { compareCaseless }     from './folio'
import { discoveredRules }     from '../../lib/content/rules'
import { renderInline }        from '../../lib/markdown/render'
import type { GlossaryFamily } from '../../lib/shared/registries'

export interface GlossaryFolioEntry {
  aliases        : readonly string[]
  definitionHtml : string
  families       : readonly GlossaryFamily[]
  href          ?: string
  initial        : string
  primaryFamily  : GlossaryFamily
  slug           : string
}

interface GlossarySource {
  href ?: string
  rule ?: string
}

interface RuleHref {
  href : string
  slug : string
}

let cached: Promise<GlossaryFolioEntry[]> | null = null

// The glossary entries the folio index and pane render, read off the
// `glossary` collection with each `rule` field resolved to its page and the
// definitions rendered, sorted caselessly by slug and derived once per build.
export function glossaryFolioEntries(): Promise<GlossaryFolioEntry[]> {
  return (cached ??= deriveEntries())
}

async function deriveEntries(): Promise<GlossaryFolioEntry[]> {
  const rules   = await discoveredRules()
  const entries = await Promise.all((await getCollection('glossary')).map(async entry => ({
    aliases        : entry.data.aliases ?? [],
    definitionHtml : await renderInline(entry.data.definition),
    families       : entry.data.families,
    href           : entryHref(entry.id, entry.data, rules),
    initial        : firstLetter(entry.id),
    primaryFamily  : entry.data.families[0],
    slug           : entry.id
  })))
  return entries.toSorted((a, b) => compareCaseless(a.slug, b.slug))
}

function entryHref(slug: string, entry: GlossarySource, rules: readonly RuleHref[]): string | undefined {
  if (entry.rule !== undefined) {
    const rule = rules.find(candidate => candidate.slug === entry.rule)
    if (rule === undefined) throw new Error(`Glossary "${slug}" names unknown rule "${entry.rule}"`)
    return rule.href
  }
  if (entry.href?.startsWith('/rules/')) {
    throw new Error(`Glossary "${slug}" hand-writes a rule URL, use the rule field instead`)
  }
  return entry.href
}

function firstLetter(slug: string): string {
  return slug.match(/[a-z]/i)?.[0]?.toUpperCase() ?? '#'
}
