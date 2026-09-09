import loader from '../../lib/sandbox/config-schema.data'

const schema = await loader.load([])

describe('sandbox config schema', () => {
  it.each(schema.rules.map(rule => rule.slug))('%s walks a hint for every facet', slug => {
    const rule = schema.rules.find(entry => entry.slug === slug)
    expect(rule?.facets.filter(facet => facet.hintHtml === '').map(facet => facet.key)).toStrictEqual([])
  })
})
