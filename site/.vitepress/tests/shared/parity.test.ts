import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'postcss'

import { glossary }          from '../../lib/glossary/entries'
import * as typingDemo       from '../../lib/landing/typing-demo'
import { discoverRuleSlugs } from '../../lib/rules/discovery'
import { rulesDir }          from '../../lib/shared/paths'
import * as registries       from '../../lib/shared/registries'

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
const slugs            = new Set(discoverRuleSlugs(rulesDir(import.meta.url)).map(r => r.slug))

describe('family registry and stylesheet parity', () => {
  it('every glossary family has a [data-family] accent, with no orphans', () => {
    expect(accentSlugs()).toEqual(glossaryFamilies)
  })

  it('FAMILY_ORDER covers FAMILY_META, and GLOSSARY_FAMILY_META adds cli and engine', () => {
    expect.soft([...registries.FAMILY_ORDER].sort()).toEqual(Object.keys(registries.FAMILY_META).sort())
    expect.soft(glossaryFamilies).toEqual([...Object.keys(registries.FAMILY_META), 'cli', 'engine'].sort())
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
