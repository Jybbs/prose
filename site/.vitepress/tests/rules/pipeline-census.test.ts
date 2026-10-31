import { passCensus } from '../../lib/rules/pipeline-census'

const PASSES = ['ordering', 'alignment', null, 'ordering', 'lint'] as const

describe('passCensus', () => {
  it('counts every pass of each family, in the order of its first pass', () => {
    expect(passCensus(PASSES)).toStrictEqual([
      { count: 2, family: 'ordering' },
      { count: 1, family: 'alignment' },
      { count: 1, family: 'lint' }
    ])
  })

  it('counts the passes up to the pointed one, a family not yet reached reading zero', () => {
    expect(passCensus(PASSES, 2)).toStrictEqual([
      { count: 1, family: 'ordering' },
      { count: 1, family: 'alignment' },
      { count: 0, family: 'lint' }
    ])
  })
})
