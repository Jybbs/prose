// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

import { provideAriaHidden } from '../../lib/composables/use-aria-hidden'
import InlineRuleLink        from '../../theme/components/rules/InlineRuleLink.vue'
import { popperStubMount }   from '../popper-stub'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ slug: 'align-equals' }]))

const MOUNT = { global: popperStubMount }

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

  it('sets the family badge before the slug', () => {
    const link = anchor(false)
    expect(link.attributes('data-family')).toBe('alignment')
    expect(link.find('svg').attributes('aria-hidden')).toBe('true')
    expect(link.find('use').attributes('href')).toBe('#nib-ladder-small')
    expect(link.text()).toBe('align-equals')
  })

  it('carries the slug the delegated popper reads', () => {
    expect(anchor(false).attributes('data-rule')).toBe('align-equals')
  })
})
