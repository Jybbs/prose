// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

import { provideAriaHidden } from '../../lib/composables/use-aria-hidden'
import InlineRuleLink        from '../../theme/components/rules/InlineRuleLink.vue'
import { popperStubMount }   from '../popper-stub'

vi.mock('../../lib/rules/rules.data', () => ({
  data: {
    bySlug: {
      'align-equals': {
        category : 'auto-fix',
        family   : 'alignment',
        href     : '/rules/alignment/align-equals',
        slug     : 'align-equals'
      }
    }
  }
}))

// `RuleTooltipPopper` is registered globally by the theme rather than imported.
const MOUNT = {
  global: {
    ...popperStubMount,
    stubs: { ...popperStubMount.stubs, RuleTooltipPopper: { template: '<div><slot /></div>' } }
  }
}

const anchor = (hidden: boolean) => mount(
  defineComponent({
    setup: () => { provideAriaHidden(hidden); return () => h(InlineRuleLink, { slug: 'align-equals' }) }
  }),
  MOUNT
).get('.rule-link')

describe('InlineRuleLink', () => {
  it('links the slug to its rule page', () => {
    expect(anchor(false).attributes('href')).toBe('/rules/alignment/align-equals')
  })

  it('stays in the tab order in ordinary prose', () => {
    expect(anchor(false).attributes('tabindex')).toBeUndefined()
  })

  it('leaves the tab order inside an aria-hidden subtree', () => {
    expect(anchor(true).attributes('tabindex')).toBe('-1')
  })
})
