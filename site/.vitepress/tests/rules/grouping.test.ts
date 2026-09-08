import { rulesDataStub } from '../rules-data-stub'

const { data } = rulesDataStub([
  { family: 'alignment', slug: 'align-equals' },
  { family: 'lint',      slug: 'line-overflow' },
  { family: 'ordering',  slug: 'alphabetize-siblings' }
])

describe('groupRules', () => {
  it('indexes every rule by its slug', () => {
    expect(Object.keys(data.bySlug).toSorted())
      .toStrictEqual(['align-equals', 'alphabetize-siblings', 'line-overflow'])
  })

  it('gives a family carrying no rule an empty list', () => {
    expect(data.byFamily.alignment.map(rule => rule.slug)).toStrictEqual(['align-equals'])
    expect(data.byFamily.docs).toStrictEqual([])
  })

  it('splits the categories and drops a family holding no rule in one', () => {
    const autoFix = data.byCategory.find(group => group.category === 'auto-fix')!
    const lint    = data.byCategory.find(group => group.category === 'lint')!
    expect(autoFix.byFamily.map(group => group.family)).toStrictEqual(['alignment', 'ordering'])
    expect(lint.byFamily.map(group => group.family)).toStrictEqual(['lint'])
  })
})
