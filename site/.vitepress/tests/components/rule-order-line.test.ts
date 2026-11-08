// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { RuleOrderLine } from '../../theme/components/rules/RuleOrderLine'

vi.mock('../../lib/rules/pipeline.data', () => ({
  data: {
    rules: [
      { after: [],    family: 'layout', slug: 'a' },
      { after: ['a'], family: null,     slug: 'b' },
      { after: [],    family: 'lint',   slug: 'c' }
    ]
  }
}))

describe('RuleOrderLine', () => {
  it.each([
    ['a', 'Pipeline order: b run after it'],
    ['b', 'Pipeline order: runs after a'],
    ['c', 'Pipeline order: no declared order']
  ])('labels %s as %j', (slug, label) => {
    expect(mount(RuleOrderLine, { props: { slug } }).attributes('aria-label')).toBe(label)
  })

  it('draws a family badge for a family rule and a dot for one without', () => {
    const marks = mount(RuleOrderLine, { props: { slug: 'a' } }).findAll('g[transform]')
    expect(marks.map(mark => mark.find('circle').exists())).toStrictEqual([true])
  })
})
