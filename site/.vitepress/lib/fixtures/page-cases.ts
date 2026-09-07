import { COMPOSITION_RULE, fixtureId } from './entry'
import { ruleSlug }                    from '../shared/rule-slug'

const ATTRIBUTE = /([\w-]+)="([^"]*)"/g
const TAG       = /<(CompositionCards|Fixture|FixtureConvergence|RuleLayout)\b([^>]*)>/g

// Holds the data that `pageCaseIds` resolves its cases against, declared
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

// Lists every fixture id a page can render, read from the component tags that
// its markdown source carries.
export function pageCaseIds(source: string, sets: PageFixtureSets): string[] {
  const ids = new Set<string>()
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
        if (attributes.rule && attributes.case) ids.add(fixtureId(attributes.rule, attributes.case))
        break
    }
  }
  return [...ids]
}
