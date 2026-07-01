import { readFileSync }  from 'node:fs'
import { fileURLToPath } from 'node:url'

import { parseFrontmatter }   from '@astrojs/markdown-remark'
import { parse as parseYaml } from 'yaml'

import { DOCS_CONTENT_DIR }          from '../shared/paths'
import { isFamily }                  from '../shared/registries'
import type { RuleFamily }           from '../shared/registries'
import { pageFiles, subdirectories } from './page'

export interface RuleRef      { caption: string, family: RuleFamily, href: string }
export interface PrimitiveRef { href: string, title: string }
export interface GlossaryRef  { definition: string, href?: string, slug: string }

export interface DocsVocab {
  glossary   : Map<string, GlossaryRef>
  primitives : Map<string, PrimitiveRef>
  rules      : Map<string, RuleRef>
}

interface GlossarySource {
  aliases    ?: string[]
  definition  : string
  href       ?: string
}

const frontmatter = (path: string): Record<string, unknown> =>
  parseFrontmatter(readFileSync(path, 'utf8')).frontmatter as Record<string, unknown>

// The rule families, primitive titles, and glossary phrases the page-body
// plugins resolve against, read from the docs tree and glossary data at config
// load, before the content collections exist. The type unions stay hand-curated
// in `registries.ts`, leaving this the runtime vocabulary the frontmatter and
// data carry.
export function discoverDocsVocab(siteRoot: URL): DocsVocab {
  const docs = fileURLToPath(new URL(DOCS_CONTENT_DIR, siteRoot))
  return {
    glossary   : glossaryPhrases(fileURLToPath(new URL('src/data/glossary.yaml', siteRoot))),
    primitives : primitiveRefs(`${docs}primitives`),
    rules      : ruleRefs(`${docs}rules`)
  }
}

function ruleRefs(dir: string): Map<string, RuleRef> {
  const out = new Map<string, RuleRef>()
  for (const family of subdirectories(dir).filter(isFamily)) {
    for (const { file, slug } of pageFiles(`${dir}/${family}`)) {
      out.set(slug, {
        caption : String(frontmatter(`${dir}/${family}/${file}`).caption ?? ''),
        family,
        href    : `/rules/${family}/${slug}`
      })
    }
  }
  return out
}

function primitiveRefs(dir: string): Map<string, PrimitiveRef> {
  const out = new Map<string, PrimitiveRef>()
  for (const { file, slug } of pageFiles(dir)) {
    out.set(slug, {
      href  : `/primitives/${slug}`,
      title : String(frontmatter(`${dir}/${file}`).title ?? slug)
    })
  }
  return out
}

// Registers each entry's own phrase and its aliases against one reference,
// throwing when a phrase resolves to two entries.
function glossaryPhrases(path: string): Map<string, GlossaryRef> {
  const source = parseYaml(readFileSync(path, 'utf8')) as Record<string, GlossarySource>
  const out    = new Map<string, GlossaryRef>()
  for (const [slug, entry] of Object.entries(source)) {
    const ref: GlossaryRef = { definition: entry.definition, href: entry.href, slug }
    for (const phrase of [slug, ...(entry.aliases ?? [])]) {
      const prior = out.get(phrase)
      if (prior && prior.slug !== slug) {
        throw new Error(`Glossary phrase "${phrase}" registered against both "${prior.slug}" and "${slug}"`)
      }
      out.set(phrase, ref)
    }
  }
  return out
}
