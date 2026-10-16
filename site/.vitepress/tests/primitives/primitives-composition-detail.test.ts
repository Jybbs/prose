// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import PrimitivesCompositionDetail from '../../theme/components/primitives/PrimitivesCompositionDetail.vue'
import RuleTooltipPopper           from '../../theme/components/rules/RuleTooltipPopper.vue'
import { popperStubMount }         from '../popper-stub'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ family: 'layout', slug: 'reflow-calls' }]))

vi.mock('../../lib/primitives/primitives-composition.data', () => ({
  data: {
    bySlug: {
      source: {
        consumedBy   : ['reflow-calls'],
        consumes     : [],
        layer        : 'base',
        name         : 'Source',
        slug         : 'source',
        summaryNodes : []
      }
    }
  }
}))

const card = () => mount(PrimitivesCompositionDetail, {
  global : { ...popperStubMount, components: { RuleTooltipPopper } },
  props  : { focused: 'source' }
})

describe('PrimitivesCompositionDetail', () => {
  it('leads each rule the focused primitive feeds with its family badge', () => {
    expect(card().get('.rule-chip use').attributes('href')).toBe('#nib-basket-small')
  })
})
