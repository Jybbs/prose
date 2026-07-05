import * as sources from '../../lib/tokens/sources'

const token = (key: string, domain: sources.Domain): sources.Token =>
  ({ blurbHtml: '', domain, href: '', key, sort: key })

describe('stripPrefix', () => {
  it.each([
    ['# fmt: off',              'off'],
    ['# prose: ignore[<slug>]', 'ignore[<slug>]'],
    ['# yapf: disable',         'disable'],
    ['--color',                 'color'],
    ['prose check',             'check']
  ])('reduces %s to its sort key', (input, expected) => {
    expect(sources.stripPrefix(input)).toBe(expected)
  })
})

describe('groupByDomain', () => {
  it('buckets by domain, both the buckets and their tokens sorted', () => {
    const tokens = [
      token('z', 'config-key'), token('a', 'cli-flag'), token('y', 'config-key'), token('b', 'cli-flag')
    ]
    expect(sources.groupByDomain(tokens)).toEqual([
      ['cli-flag',   [token('a', 'cli-flag'), token('b', 'cli-flag')]],
      ['config-key', [token('y', 'config-key'), token('z', 'config-key')]]
    ])
  })

  it('does not mutate its input', () => {
    const input = [token('b', 'cli-flag'), token('a', 'cli-flag')]
    sources.groupByDomain(input)
    expect(input.map(t => t.key)).toEqual(['b', 'a'])
  })
})
