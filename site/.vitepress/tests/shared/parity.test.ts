import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'postcss'

import { glossary }           from '../../lib/glossary/entries'
import * as typingDemo        from '../../lib/landing/typing-demo'
import { discoverPrimitives } from '../../lib/primitives/discovery'
import { discoverRuleSlugs }  from '../../lib/rules/discovery'
import * as paths             from '../../lib/shared/paths'
import * as registries        from '../../lib/shared/registries'
import * as ruleSchema        from '../../lib/shared/rule-schema'

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
const discovered       = discoverRuleSlugs(paths.rulesDir(import.meta.url))
const slugs            = new Set(discovered.map(r => r.slug))

const schema   = ruleSchema.proseSchema(paths.repoRoot(import.meta.url))
const ruleDefs = ruleSchema.ruleDefsOf(schema)

const ownFacetKeysOf = (slug: string): string[] =>
  ruleSchema.ownFacetKeys(schema.$defs, ruleDefs[slug]?.default ?? {})

// A rule heads its facets once it carries two or more of its own, leaving one
// with a single facet to the configuration table.
const headsFacets = (keys: string[]): boolean => keys.length > 1

const facetSlot = (family: string, slug: string): string => {
  const page = fs.readFileSync(path.join(paths.rulesDir(import.meta.url), family, `${slug}.md`), 'utf8')
  return page.split('<template #facets>')[1]?.split('</template>')[0] ?? ''
}

const facetedPages = discovered.flatMap(rule => {
  const keys = ownFacetKeysOf(rule.slug)
  return headsFacets(keys) ? [{ family: rule.family, keys, slug: rule.slug }] : []
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
    expect(discoverPrimitives(paths.primitivesDir(import.meta.url)).map(p => p.slug).toSorted())
      .toStrictEqual([...registries.PRIMITIVE_SLUGS].toSorted())
  })
})

describe('rule page and facet parity', () => {
  it('reaches a page for every rule the schema gives two or more facets of its own', () => {
    expect(facetedPages.map(page => page.slug).toSorted())
      .toStrictEqual(Object.keys(ruleDefs).filter(slug => headsFacets(ownFacetKeysOf(slug))).toSorted())
  })

  it.each(facetedPages)('$slug heads each of its own facets in its facets slot', ({ family, keys, slug }) => {
    const slot = facetSlot(family, slug)
    expect(keys.filter(key => !slot.includes(`### \`${key}\``))).toStrictEqual([])
  })

  it('gives a facets slot to no rule carrying fewer than two of its own facets', () => {
    const slotted = discovered.filter(rule => facetSlot(rule.family, rule.slug) !== '')
    expect(slotted.map(rule => rule.slug).toSorted())
      .toStrictEqual(facetedPages.map(page => page.slug).toSorted())
  })

  it.each(facetedPages)('$slug heads no facet the schema dropped', ({ family, keys, slug }) => {
    const headed = [...facetSlot(family, slug).matchAll(/^### `([^`]+)`$/gm)].map(([, key]) => key)
    expect(headed.filter(key => !keys.includes(key))).toStrictEqual([])
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
