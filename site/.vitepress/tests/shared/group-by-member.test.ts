import { groupByMember } from '../../lib/shared/group-by-member'

const MEMBERS = ['a', 'b', 'c'] as const

describe('groupByMember', () => {
  it('groups each item under the key it yields', () => {
    const out = groupByMember([{ id: 1, tag: 'a' }, { id: 2, tag: 'a' }], i => i.tag, MEMBERS)
    expect(out.a.map(i => i.id)).toEqual([1, 2])
  })

  it('gives a member no item names an empty list rather than undefined', () => {
    const out = groupByMember([{ id: 1, tag: 'a' }], i => i.tag, MEMBERS)
    expect(out.b).toEqual([])
    expect(out.c).toEqual([])
  })

  it('returns an entry per member from an empty input', () => {
    expect(groupByMember([], (i: { tag: 'a' | 'b' | 'c' }) => i.tag, MEMBERS))
      .toEqual({ a: [], b: [], c: [] })
  })
})
