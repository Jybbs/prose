import { BADGE, orderLine, orderRelation } from '../../lib/rules/order-line'

const RULES = [
  { after: [],         family: 'layout' as const,   slug: 'a' },
  { after: [],         family: 'layout' as const,   slug: 'b' },
  { after: ['a', 'b'], family: 'ordering' as const, slug: 'c' },
  { after: ['c'],      family: null,                slug: 'd' },
  { after: ['c'],      family: 'lint' as const,     slug: 'e' }
]

describe('orderLine', () => {
  const line = orderLine(RULES, 'c', 100)

  it('marks the rule at its own pass along the line', () => {
    expect(line.here).toBe(BADGE / 2 + 2 * (100 - BADGE) / 4)
  })

  it('sets the rules it runs after above the line and the rules running after it below', () => {
    expect(line.earlier.map(mark => mark.slug)).toStrictEqual(['a', 'b'])
    expect(line.later.map(mark => mark.slug)).toStrictEqual(['d', 'e'])
    expect(line.earlier.every(mark => mark.y + BADGE <= line.lineY)).toBe(true)
    expect(line.later.every(mark => mark.y >= line.lineY)).toBe(true)
  })

  it('lifts a badge one row where it would overlap the one before it', () => {
    const [first, second] = orderLine(RULES, 'c', 40).earlier
    expect(second.y).toBe(first.y - BADGE - 2)
    expect(new Set(line.earlier.map(mark => mark.y)).size).toBe(1)
  })

  it('opens a further row where every row already holds a badge within reach', () => {
    const crowded = [...RULES, { after: ['a', 'b', 'c'], family: null, slug: 'f' }]
    const marks   = orderLine(crowded, 'f', 30).earlier
    const clash   = marks.some((mark, nth) => marks.slice(nth + 1).some(other =>
      other.y === mark.y && Math.abs(other.x - mark.x) < BADGE))
    expect(new Set(marks.map(mark => mark.y)).size).toBe(3)
    expect(clash).toBe(false)
  })

  it('keeps one height whatever the rule', () => {
    expect(new Set(RULES.map(rule => orderLine(RULES, rule.slug, 100).height)).size).toBe(1)
  })
})

describe('orderRelation', () => {
  it.each([
    ['c', 'c', 'self'],
    ['c', 'a', 'earlier'],
    ['c', 'd', 'later'],
    ['a', 'b', undefined]
  ])('names %s against %s as %s', (chosen, rule, relation) => {
    const find = (slug: string) => RULES.find(entry => entry.slug === slug)!
    expect(orderRelation(find(chosen), find(rule))).toBe(relation)
  })
})
