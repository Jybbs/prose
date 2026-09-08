import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'postcss'

import { glossary }                          from '../../lib/glossary/entries'
import * as typingDemo                       from '../../lib/landing/typing-demo'
import { discoverPrimitives }                from '../../lib/primitives/discovery'
import { discoverRuleSlugs }                 from '../../lib/rules/discovery'
import { primitivesDir, repoRoot, rulesDir } from '../../lib/shared/paths'
import * as registries                       from '../../lib/shared/registries'
import * as ruleSchema                       from '../../lib/shared/rule-schema'

const styles = (name: string): string =>
  fs.readFileSync(path.join(import.meta.dirname, '..', '..', 'theme', 'styles', name), 'utf8')

const accentSlugs = (): string[] => {
  const slugs = new Set<string>()
  parse(styles('accents.css')).walkRules(rule => {
    for (const m of rule.selector.matchAll(/\[data-family="([a-z]+)"\]/g)) slugs.add(m[1])
  })
  return [...slugs].sort()
}

const glossaryFamilies = Object.keys(registries.GLOSSARY_FAMILY_META).sort()
const discovered       = discoverRuleSlugs(rulesDir(import.meta.url))
const slugs            = new Set(discovered.map(r => r.slug))

const schema   = ruleSchema.proseSchema(repoRoot(import.meta.url))
const ruleDefs = ruleSchema.ruleDefsOf(schema)

const facetedPages = discovered.flatMap(rule => {
  const keys = ruleSchema.ownFacetKeys(schema.$defs, ruleDefs[rule.slug]?.default ?? {})
  return keys.length > 1 ? [{ family: rule.family, keys, slug: rule.slug }] : []
})

describe('family registry and stylesheet parity', () => {
  it('every glossary family has a [data-family] accent, with no orphans', () => {
    expect(accentSlugs()).toStrictEqual(glossaryFamilies)
  })

  it('FAMILY_ORDER covers FAMILY_META, and GLOSSARY_FAMILY_META adds cli and engine', () => {
    expect.soft([...registries.FAMILY_ORDER].sort()).toStrictEqual(Object.keys(registries.FAMILY_META).sort())
    expect.soft(glossaryFamilies).toStrictEqual([...Object.keys(registries.FAMILY_META), 'cli', 'engine'].sort())
  })
})

describe('primitive registry and page parity', () => {
  it('every PRIMITIVE_SLUGS entry has a page, with no orphans', () => {
    expect(discoverPrimitives(primitivesDir(import.meta.url)).map(p => p.slug).toSorted())
      .toStrictEqual([...registries.PRIMITIVE_SLUGS].toSorted())
  })
})

describe('rule page and facet parity', () => {
  it('reaches a page for every rule the schema gives two or more facets of its own', () => {
    expect(facetedPages.map(page => page.slug).toSorted()).toStrictEqual(
      Object.entries(ruleDefs)
        .flatMap(([slug, def]) => ruleSchema.ownFacetKeys(schema.$defs, def.default).length > 1 ? [slug] : [])
        .toSorted()
    )
  })

  it.each(facetedPages)('$slug heads each of its own facets in its facets slot', ({ family, keys, slug }) => {
    const page  = fs.readFileSync(path.join(rulesDir(import.meta.url), family, `${slug}.md`), 'utf8')
    const slot  = page.split('<template #facets>')[1]?.split('</template>')[0] ?? ''
    expect(keys.filter(key => !slot.includes(`### \`${key}\``))).toStrictEqual([])
  })
})

describe('glossary rule resolution', () => {
  const ruleEntries = Object.entries(glossary).flatMap(([name, entry]) =>
    entry.rule ? [{ name, rule: entry.rule }] : []
  )

  it.each(ruleEntries)('$name resolves rule $rule to a discovered slug', ({ rule }) => {
    expect(slugs.has(rule)).toBe(true)
  })
})

describe('typing-demo rule resolution', () => {
  it.each([...typingDemo.RULES])('demo rule %s is a discovered slug', rule => {
    expect(slugs.has(rule)).toBe(true)
  })
})
