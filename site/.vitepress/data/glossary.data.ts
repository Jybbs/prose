import { defineLoader } from 'vitepress'

import { glossary }    from '../lib/glossary/glossary-data'
import { getRenderer } from '../lib/markdown/renderer'

type GlossaryFamily = 'alignment' | 'docs' | 'engine' | 'formatting' | 'lint' | 'ordering'

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

const ALIGNMENT_KEYS  = ['align', 'aligner', 'alignment', 'singleton', 'max-shift', 'match', 'walrus']
const DOCS_KEYS       = ['docstring', 'pep 257', 'pep-257', 'structured section', 'docstring-line-length']
const ENGINE_KEYS     = ['ast', 'pipeline', 'source', 'reparse', 'suppression', 'ruff', 'parser', 'binding', 'walker', 'gitignore', 'fixture', 'idempotent', 'stdin', 'severity', 'diagnostic', 'workflow', 'ndjson', 'applicability', 'kebab', 'ruleid', 'rule id', 'target-version']
const FORMATTING_KEYS = ['fmt:', 'prose:', '--ignore', '--select', 'blank line', 'collection', 'atomic', 'trailing comma', 'one-per-line', 'code-line-length', 'comprehension', 'auto-fix', 'f-string']
const LINT_KEYS       = ['lint', 'binding', 'loose-constants', 'single-use', 'forward reference', 'legacy-union', 'unused-future', 'annotation', 'type_checking', 'pep 604', 'pep-604', 'pep 749', 'pep-749']
const ORDERING_KEYS   = ['alphabetize', 'order', 'orderer', 'dataclass', 'enum', 'typeddict', 'pydantic', 'dunder', 'decorator']

const FAMILY_BADGES: Record<GlossaryFamily, string> = {
  alignment  : '\u{1FA9C}',
  docs       : '\u{1F4F0}',
  engine     : '\u{1F989}',
  formatting : '\u{1FAB6}',
  lint       : '\u{1F9F6}',
  ordering   : '\u{1FA89}'
}

declare const data: GlossaryData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<GlossaryData> {
    const md      = await getRenderer()
    const entries : Record<string, RenderedGlossaryEntry> = {}

    for (const [slug, entry] of Object.entries(glossary)) {
      const aliases = entry.aliases ?? []
      const family  = classify(slug, aliases)
      entries[slug] = {
        aliases,
        definitionHtml : md.renderInline(entry.definition),
        family,
        familyBadge    : FAMILY_BADGES[family],
        familyLabel    : family,
        href           : entry.href,
        initial        : firstLetter(slug),
        slug
      }
    }

    return { entries }
  }
})

function classify(slug: string, aliases: readonly string[]): GlossaryFamily {
  const hay = [slug, ...aliases].map(s => s.toLowerCase())
  if (matchesAny(hay, ALIGNMENT_KEYS))  return 'alignment'
  if (matchesAny(hay, ORDERING_KEYS))   return 'ordering'
  if (matchesAny(hay, DOCS_KEYS))       return 'docs'
  if (matchesAny(hay, LINT_KEYS))       return 'lint'
  if (matchesAny(hay, ENGINE_KEYS))     return 'engine'
  if (matchesAny(hay, FORMATTING_KEYS)) return 'formatting'
  return 'formatting'
}

function firstLetter(slug: string): string {
  return slug.match(/[a-z]/i)?.[0].toUpperCase() ?? '#'
}

function matchesAny(strings: readonly string[], keys: readonly string[]): boolean {
  return strings.some(s => keys.some(k => s.includes(k)))
}
