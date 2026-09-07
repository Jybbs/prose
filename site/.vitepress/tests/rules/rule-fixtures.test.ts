import { readFixtureToggle } from '../../lib/fixtures/toggle'
import * as walker           from '../../lib/fixtures/walker'
import { readRuleFixtures }  from '../../lib/rules/rule-fixtures'
import { crateDir }          from '../../lib/shared/paths'

const crate = crateDir(import.meta.url)
const data  = await readRuleFixtures(crate)
const rules = Object.entries(data)

const inputPath = (rule: string, caseName: string): string =>
  [...walker.walkFixtures(crate)].find(e => e.rule === rule && e.caseName === caseName)!.inputPath

describe('readRuleFixtures', () => {
  it('registers a canonical case for every rule it lists', () => {
    expect(rules.length).toBeGreaterThan(0)
    expect(rules.every(([, set]) => set.canonical.length > 0)).toBe(true)
  })

  it('leaves out a directory whose cases register no canonical one', () => {
    expect(data.composition).toBeUndefined()
  })

  it('renders every example title to HTML, leaving no backtick behind', () => {
    const titles = rules.flatMap(([, set]) => set.examples.map(example => example.titleHtml))
    expect(titles.length).toBeGreaterThan(0)
    expect(titles.some(title => title.includes('<code>'))).toBe(true)
    expect(titles.every(title => !title.includes('`'))).toBe(true)
  })

  it('names each example once per rule', () => {
    const duplicated = rules
      .filter(([, set]) => new Set(set.examples.map(ex => ex.case)).size !== set.examples.length)
      .map(([rule]) => rule)
    expect(duplicated).toEqual([])
  })

  it('leaves the canonical case out of the examples', () => {
    expect(rules.every(([, set]) => !set.examples.some(ex => ex.case === set.canonical))).toBe(true)
  })

  it('orders the transforming examples ahead of the ones that change nothing', async () => {
    const [rule, set] = rules.find(([, entry]) => entry.examples.length > 3)!
    const toggles     = await Promise.all(set.examples.map(async example =>
      (await readFixtureToggle(inputPath(rule, example.case))).hasToggle))
    expect(toggles).toEqual([...toggles].sort((a, b) => Number(b) - Number(a)))
  })
})
