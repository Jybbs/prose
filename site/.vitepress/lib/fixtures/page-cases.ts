import { caseNames, COMPOSITION_RULE, fixtureId, gaugeId } from './entry'
import { ruleSlug }                                        from '../shared/rule-slug'

const ATTRIBUTE = /([\w-]+)="([^"]*)"/g
const TAG       = /<(CompositionCards|CompositionTrail|Fixture|FixtureCards|FixtureConvergence|RuleLayout)\b([^>]*)>/g

// Names one gauge a page draws, by its id, its case, and the setting it steps.
export interface GaugeRequest {
  case    : string
  id      : string
  rule    : string
  setting : string
}

// Holds every fixture id a page can render beside the ids its `Fixture`,
// `FixtureCards`, `FixtureConvergence`, and `CompositionTrail` tags embed inline,
// which the card lists on that page leave out, with the gauges a `Fixture` naming
// a `setting` draws and the trails each `CompositionTrail` draws.
export interface PageCases {
  gauges : GaugeRequest[]
  ids    : string[]
  inline : string[]
  trails : string[]
}

// Holds the data that `readPageCases` resolves its cases against, declared
// structurally so it names neither loader's own type.
export interface PageFixtureSets {
  composition: {
    byRule : Record<string, readonly string[]>
    cases  : readonly { case: string }[]
  }
  ruleFixtures: Record<string, { canonical: string, examples: readonly string[] }>
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
    ...set.examples.map(caseName => fixtureId(rule, caseName)),
    ...compositionIds(sets, rule)
  ]
}

// Reads the fixture ids a page can render and the ones it embeds inline from
// the component tags its markdown source carries.
export function readPageCases(source: string, sets: PageFixtureSets): PageCases {
  const ids    = new Set<string>()
  const inline = new Set<string>()
  const gauges = new Map<string, GaugeRequest>()
  const trails = new Set<string>()
  const embed  = (id: string): void => { ids.add(id); inline.add(id) }
  for (const [, tag, rawAttributes] of source.matchAll(TAG)) {
    const attributes = Object.fromEntries(
      rawAttributes.matchAll(ATTRIBUTE).map(([, name, value]) => [name, value])
    )
    switch (tag) {
      case 'CompositionCards':
        for (const id of compositionIds(sets, attributes.rule)) ids.add(id)
        break
      case 'CompositionTrail':
        if (attributes.case) {
          const id = fixtureId(COMPOSITION_RULE, attributes.case)
          embed(id)
          trails.add(id)
        }
        break
      case 'RuleLayout':
        for (const id of ruleLayoutIds(sets, attributes.rule)) ids.add(id)
        break
      case 'Fixture':
      case 'FixtureConvergence': {
        const { case: caseName, rule, setting } = attributes
        if (!rule || !caseName) break
        embed(fixtureId(rule, caseName))
        if (setting) {
          const id = gaugeId(rule, caseName, setting)
          gauges.set(id, { case: caseName, id, rule, setting })
        }
        break
      }
      case 'FixtureCards':
        if (attributes.rule && attributes.cases) {
          for (const name of caseNames(attributes.cases)) embed(fixtureId(attributes.rule, name))
        }
        break
    }
  }
  return { gauges: [...gauges.values()], ids: [...ids], inline: [...inline], trails: [...trails] }
}
