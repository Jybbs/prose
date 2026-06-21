import fs   from 'node:fs'
import path from 'node:path'

import postcss from 'postcss'

import { glossary }                                        from '../lib/glossary/entries'
import { discoverRuleSlugs }                               from '../lib/rules/discovery'
import { resolveToken }                                    from '../lib/shared/css-token'
import { rulesDir }                                        from '../lib/shared/paths'
import { FAMILY_META, FAMILY_ORDER, GLOSSARY_FAMILY_META } from '../lib/shared/registries'

const styles = (name: string): string =>
  fs.readFileSync(path.join(import.meta.dirname, '..', 'theme', 'styles', name), 'utf8')

const familyTokenSlugs = (): string[] => {
  const slugs: string[] = []
  postcss.parse(styles('tokens.css')).walkDecls(/^--prose-c-family-/, decl =>
    void slugs.push(decl.prop.replace('--prose-c-family-', '')))
  return slugs.sort()
}

const accentSlugs = (): string[] => {
  const slugs = new Set<string>()
  postcss.parse(styles('accents.css')).walkRules(rule => {
    for (const m of rule.selector.matchAll(/\[data-family="([a-z]+)"\]/g)) slugs.add(m[1])
  })
  return [...slugs].sort()
}

const glossaryFamilies = Object.keys(GLOSSARY_FAMILY_META).sort()

describe('family registry and stylesheet parity', () => {
  it('every glossary family has a --prose-c-family token, with no orphans', () => {
    expect(familyTokenSlugs()).toEqual(glossaryFamilies)
  })

  it('every glossary family has a [data-family] accent, with no orphans', () => {
    expect(accentSlugs()).toEqual(glossaryFamilies)
  })

  it.each(glossaryFamilies)('resolveToken resolves prose-c-family-%s', (family) => {
    expect(resolveToken(`prose-c-family-${family}`)).not.toBe('')
  })

  it('FAMILY_ORDER covers FAMILY_META, and GLOSSARY_FAMILY_META adds cli and engine', () => {
    expect.soft([...FAMILY_ORDER].sort()).toEqual(Object.keys(FAMILY_META).sort())
    expect.soft(glossaryFamilies).toEqual([...Object.keys(FAMILY_META), 'cli', 'engine'].sort())
  })
})

describe('glossary rule resolution', () => {
  const slugs       = new Set(discoverRuleSlugs(rulesDir(import.meta.url)).map(r => r.slug))
  const ruleEntries = Object.entries(glossary).flatMap(([name, entry]) =>
    entry.rule ? [{ name, rule: entry.rule }] : []
  )

  it.each(ruleEntries)('$name resolves rule $rule to a discovered slug', ({ rule }) => {
    expect(slugs.has(rule)).toBe(true)
  })
})
