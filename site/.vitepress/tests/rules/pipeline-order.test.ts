// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import PipelineOrder       from '../../theme/components/rules/PipelineOrder.vue'
import RuleTooltipPopper   from '../../theme/components/rules/RuleTooltipPopper.vue'
import { popperStubMount } from '../popper-stub'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ family: 'layout', slug: 'reflow-calls' }]))

vi.mock('../../lib/rules/pipeline.data', () => ({
  data: {
    rules: [
      { documented: true,  family: 'layout', position: 1, slug: 'reflow-calls',   title: 'reflow-calls' },
      { documented: false, family: null,     position: 2, slug: 'unwritten-rule', title: 'unwritten-rule' }
    ]
  }
}))

const entries = () => mount(PipelineOrder, {
  global: { ...popperStubMount, components: { RuleTooltipPopper } }
}).findAll('.pipeline-order-entry')

describe('PipelineOrder', () => {
  it('ends a documented rule\'s entry on its family badge', () => {
    expect(entries()[0].get('use').attributes('href')).toBe('#nib-basket-small')
  })

  it('ends an undocumented rule\'s entry on a plain dot', () => {
    const entry = entries()[1]
    expect(entry.find('use').exists()).toBe(false)
    expect(entry.get('.pipeline-order-glyph').text()).toBe('·')
  })
})
