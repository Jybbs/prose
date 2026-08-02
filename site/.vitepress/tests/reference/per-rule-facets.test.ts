// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import InlineProse          from '../../theme/components/base/InlineProse.vue'
import InlineRuleLink       from '../../theme/components/rules/InlineRuleLink.vue'
import PerRuleFacets        from '../../theme/components/reference/PerRuleFacets.vue'
import { expectAccessible } from '../axe'

vi.mock('../../lib/rules/rules.data', () => ({ data: {} }))

vi.mock('../../lib/reference/facets.data', () => ({
  data: [
    {
      badge : '',
      family: 'generic',
      label : 'Generic',
      rules : [
        { rule: 'every rule', facets: [{ default: 'true', key: 'enabled', meaningNodes: [{ kind: 'text', text: 'Toggle the rule.' }], type: 'bool' }] }
      ]
    },
    {
      badge : '🧺',
      family: 'layout',
      label : 'Layout',
      rules : [
        { rule: 'call-layout', facets: [{ default: '3', key: 'max-args', meaningNodes: [{ kind: 'text', text: 'Explode a call.' }], type: 'positive int | false' }] },
        { rule: 'collection-layout', facets: [
          { default: 'true', key: 'keep-multiline-literals', meaningNodes: [{ kind: 'text', text: 'Join with ' }, { kind: 'code', text: 'false' }, { kind: 'text', text: '.' }], type: 'bool' },
          { default: '8', key: 'max-atomics', meaningNodes: [{ kind: 'text', text: 'Keep short.' }], type: 'positive int | false' }
        ] }
      ]
    }
  ]
}))

const mountFacets = () =>
  mount(PerRuleFacets, { global: { components: { InlineProse }, stubs: { InlineRuleLink: true } } })

describe('PerRuleFacets', () => {
  it('renders one collapsible head per family, counting facets across its rules', () => {
    const heads = mountFacets().findAll('.per-rule-facets-head')
    expect(heads).toHaveLength(2)
    expect(heads[0].get('.per-rule-facets-label').text()).toContain('Generic')
    expect(heads[0].get('.per-rule-facets-count').text()).toBe('1 facet')
    expect(heads[1].get('.per-rule-facets-count').text()).toBe('3 facets')
  })

  it('starts collapsed and expands its section on click', async () => {
    const head = mountFacets().findAll('.per-rule-facets-head')[0]
    expect(head.attributes('aria-expanded')).toBe('false')
    await head.trigger('click')
    expect(head.attributes('aria-expanded')).toBe('true')
  })

  it('nests facets under a rule chip, keeping a generic scope as plain text', () => {
    const w = mountFacets()
    expect(w.get('.per-rule-facets-scope').text()).toBe('every rule')
    expect(w.findAllComponents(InlineRuleLink).map(c => c.props('slug'))).toEqual(['call-layout', 'collection-layout'])
    expect(w.findAll('.per-rule-facets-key').map(k => k.text())).toEqual(['enabled', 'max-args', 'keep-multiline-literals', 'max-atomics'])
  })

  it('renders each facet type, default, and rendered meaning', () => {
    const w = mountFacets()
    const maxArgs = w.findAll('.per-rule-facets-entry')[1]
    expect(maxArgs.get('.per-rule-facets-type').text()).toBe('positive int | false')
    expect(maxArgs.get('.per-rule-facets-default-value').text()).toBe('3')
    expect(w.findAll('.per-rule-facets-entry')[2].get('.per-rule-facets-meaning').html()).toContain('<code>false</code>')
  })

  it('carries the family accent through data-family', () => {
    expect(mountFacets().findAll('.per-rule-facets-section')[1].attributes('data-family')).toBe('layout')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountFacets().html())
  })
})
