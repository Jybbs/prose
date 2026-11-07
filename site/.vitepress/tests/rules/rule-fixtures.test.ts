import { readFixtureToggle } from '../../lib/fixtures/toggle'
import { readRuleFixtures }  from '../../lib/rules/rule-fixtures'
import { CASES, CRATE }      from '../corpus'

const data  = await readRuleFixtures(CRATE)
const rules = Object.entries(data)

const inputPath = (rule: string, caseName: string): string =>
  CASES.find(e => e.rule === rule && e.caseName === caseName)!.inputPath

describe('readRuleFixtures', () => {
  it('registers a canonical case for every rule it lists', () => {
    expect(rules.length).toBeGreaterThan(0)
    expect(rules.every(([, set]) => set.canonical.length > 0)).toBe(true)
  })

  it('leaves out a directory whose cases register no canonical one', () => {
    expect(data.composition).toBeUndefined()
  })

  it('names each example once per rule', () => {
    const duplicated = rules
      .filter(([, set]) => new Set(set.examples).size !== set.examples.length)
      .map(([rule]) => rule)
    expect(duplicated).toStrictEqual([])
  })

  it('leaves the canonical case out of the examples', () => {
    expect(rules.every(([, set]) => !set.examples.includes(set.canonical))).toBe(true)
  })

  it('orders the transforming examples ahead of the ones that change nothing', async () => {
    const [rule, set] = rules.find(([, entry]) => entry.examples.length > 3)!
    const toggles     = await Promise.all(set.examples.map(async caseName =>
      (await readFixtureToggle(inputPath(rule, caseName))).hasToggle))
    expect(toggles).toStrictEqual([...toggles].sort((a, b) => Number(b) - Number(a)))
  })
})
