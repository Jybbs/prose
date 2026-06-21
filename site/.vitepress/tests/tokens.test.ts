import { groupByDomain, sortedTokens, stripPrefix, type Domain, type Token } from '../lib/tokens/sources'

const token = (key: string, domain: Domain): Token =>
  ({ blurbHtml: '', domain, href: '', key, sort: key })

describe('stripPrefix', () => {
  it.each([
    ['# fmt: off',              'off'],
    ['# prose: ignore[<slug>]', 'ignore[<slug>]'],
    ['# yapf: disable',         'disable'],
    ['--color',                 'color'],
    ['prose check',             'check']
  ])('reduces %s to its sort key', (input, expected) => {
    expect(stripPrefix(input)).toBe(expected)
  })
})

describe('sortedTokens', () => {
  const tokens = [token('b', 'cli-flag'), token('a', 'suppression'), token('c', 'config-key')]

  it('sorts by sort key by default', () => {
    expect(sortedTokens(tokens).map(t => t.key)).toEqual(['a', 'b', 'c'])
  })

  it('sorts by domain then sort key in domain mode', () => {
    expect(sortedTokens(tokens, 'domain').map(t => t.domain))
      .toEqual(['cli-flag', 'config-key', 'suppression'])
  })

  it('breaks domain ties by sort key', () => {
    const same = [token('z', 'cli-flag'), token('a', 'cli-flag')]
    expect(sortedTokens(same, 'domain').map(t => t.key)).toEqual(['a', 'z'])
  })

  it('does not mutate its input', () => {
    const input = [token('b', 'cli-flag'), token('a', 'cli-flag')]
    sortedTokens(input)
    expect(input.map(t => t.key)).toEqual(['b', 'a'])
  })
})

describe('groupByDomain', () => {
  it('buckets by domain, both the buckets and their tokens sorted', () => {
    const tokens = [
      token('z', 'config-key'), token('a', 'cli-flag'), token('y', 'config-key'), token('b', 'cli-flag')
    ]
    expect(groupByDomain(tokens)).toEqual([
      ['cli-flag',   [token('a', 'cli-flag'), token('b', 'cli-flag')]],
      ['config-key', [token('y', 'config-key'), token('z', 'config-key')]]
    ])
  })
})
