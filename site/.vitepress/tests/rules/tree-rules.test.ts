// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import InlineRuleLink from '../../theme/components/rules/InlineRuleLink.vue'
import TreeRules      from '../../theme/components/rules/TreeRules.vue'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([
    { slug: 'align-equals' },
    { family: 'layout', slug: 'reflow-calls' }
  ]))

vi.mock('../../lib/rules/pipeline.data', () => ({
  data: {
    rules: [
      { documented: true, preservesTree: true, slug: 'align-equals' },
      { documented: false, preservesTree: true, slug: 'unwritten-rule' },
      { documented: true, preservesTree: false, slug: 'reflow-calls' }
    ]
  }
}))

describe('TreeRules', () => {
  it.each([
    ['kept',    ['align-equals']],
    ['changed', ['reflow-calls']]
  ] as const)('lists the rules whose tree is %s', (tree, slugs) => {
    const rendered = mount(TreeRules, { global: { components: { InlineRuleLink } }, props: { tree } })
    expect(rendered.findAll('[data-rule]').map(link => link.attributes('data-rule'))).toStrictEqual(slugs)
  })

  it('renders an undocumented rule as its plain slug', () => {
    const rendered = mount(TreeRules, { global: { components: { InlineRuleLink } }, props: { tree: 'kept' } })
    expect(rendered.findAll('li > code').map(code => code.text())).toStrictEqual(['unwritten-rule'])
  })
})
