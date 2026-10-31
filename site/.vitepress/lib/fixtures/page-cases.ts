import { COMPOSITION_RULE, fixtureId } from './entry'
import { ruleSlug }                    from '../shared/rule-slug'

const ATTRIBUTE = /([\w-]+)="([^"]*)"/g
const TAG       = /<(CompositionCards|Fixture|FixtureConvergence|RuleLayout)\b([^>]*)>/g

// Holds every fixture id a page can render beside the ids its `Fixture` and
// `FixtureConvergence` tags embed inline, which the card lists on that page
// leave out.
export interface PageCases {
  ids    : string[]
  inline : string[]
}

// Holds the data that `readPageCases` resolves its cases against, declared
// structurally so it names neither loader's own type.
export interface PageFixtureSets {
  composition: {
    byRule : Record<string, readonly string[]>
    cases  : readonly { case: string }[]
  }
  ruleFixtures: Record<string, { canonical: string, examples: readonly { case: string }[] }>
}

// Lists the composition cases one rule takes part in, falling back to every
// previewable case where no rule narrows the set.
function compositionIds(sets: PageFixtureSets, rule: string | undefined): string[] {
  const cases = rule === undefined
    ? sets.composition.cases.map(entry => entry.case)
    : sets.composition.byRule[ruleSlug(rule)] ?? []
  return cases.map(caseName => fixtureId(COMPOSITION_RULE, caseName))
}

// Lists the cases a `RuleLayout` renders for one rule, covering its canonical
// case, its previewable examples, and the composition cases it takes part in.
function ruleLayoutIds(sets: PageFixtureSets, rule: string | undefined): string[] {
  if (rule === undefined) return []
  const set = sets.ruleFixtures[rule]
  if (set === undefined) return []
  return [
    fixtureId(rule, set.canonical),
    ...set.examples.map(example => fixtureId(rule, example.case)),
    ...compositionIds(sets, rule)
  ]
}

// Reads the fixture ids a page can render and the ones it embeds inline from
// the component tags its markdown source carries.
export function readPageCases(source: string, sets: PageFixtureSets): PageCases {
  const ids    = new Set<string>()
  const inline = new Set<string>()
  for (const [, tag, rawAttributes] of source.matchAll(TAG)) {
    const attributes = Object.fromEntries(
      rawAttributes.matchAll(ATTRIBUTE).map(([, name, value]) => [name, value])
    )
    switch (tag) {
      case 'CompositionCards':
        for (const id of compositionIds(sets, attributes.rule)) ids.add(id)
        break
      case 'RuleLayout':
        for (const id of ruleLayoutIds(sets, attributes.rule)) ids.add(id)
        break
      case 'Fixture':
      case 'FixtureConvergence':
        if (attributes.rule && attributes.case) {
          const id = fixtureId(attributes.rule, attributes.case)
          ids.add(id)
          inline.add(id)
        }
        break
    }
  }
  return { ids: [...ids], inline: [...inline] }
}
