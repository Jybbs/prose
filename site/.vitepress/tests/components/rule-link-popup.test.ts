// @vitest-environment happy-dom
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick }               from 'vue'

import RuleLinkPopup  from '../../theme/components/rules/RuleLinkPopup.vue'

const DELAY = { hide: 120, show: 80 }

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ family: 'lint', slug: 'line-overflow' }]))

const stubs = { teleport: true, RuleCard: { template: '<div class="rule-card"><span class="rule-card-slug" /></div>' } }

const after = async (ms: number): Promise<void> => {
  vi.advanceTimersByTime(ms)
  await nextTick()
}

const cardOf = (wrapper: VueWrapper) => wrapper.find('.rule-card')

const pointer = (target: Element, type: 'mouseout' | 'mouseover'): void => {
  target.dispatchEvent(new MouseEvent(type, { bubbles: true }))
}

// Mounts the popup beside one rule link and opens its card through the link.
async function opened(): Promise<{ link: Element, wrapper: VueWrapper }> {
  document.body.innerHTML = '<a class="rule-link" data-rule="line-overflow"></a>'
  const link    = document.querySelector('a')!
  const wrapper = mount(RuleLinkPopup, { attachTo: document.body, global: { stubs } })
  pointer(link, 'mouseover')
  await after(DELAY.show)
  return { link, wrapper }
}

describe('RuleLinkPopup', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('closes the card once the pointer leaves its link', async () => {
    const { link, wrapper } = await opened()
    expect(cardOf(wrapper).exists()).toBe(true)
    pointer(link, 'mouseout')
    await after(DELAY.hide)
    expect(cardOf(wrapper).exists()).toBe(false)
  })

  it('keeps the card open while the pointer rests on it, and closes it once the pointer leaves', async () => {
    const { link, wrapper } = await opened()
    pointer(link, 'mouseout')
    pointer(cardOf(wrapper).element, 'mouseover')
    await after(DELAY.hide * 2)
    expect(cardOf(wrapper).exists()).toBe(true)
    pointer(cardOf(wrapper).element, 'mouseout')
    await after(DELAY.hide)
    expect(cardOf(wrapper).exists()).toBe(false)
  })

  it('keeps the card open as the pointer crosses between the elements inside it', async () => {
    const { link, wrapper } = await opened()
    pointer(link, 'mouseout')
    pointer(cardOf(wrapper).element, 'mouseover')
    pointer(cardOf(wrapper).element, 'mouseout')
    pointer(cardOf(wrapper).get('.rule-card-slug').element, 'mouseover')
    await after(DELAY.hide * 2)
    expect(cardOf(wrapper).exists()).toBe(true)
  })

  it('ignores a pointer moving over the rest of the page', async () => {
    const { wrapper } = await opened()
    pointer(document.body, 'mouseover')
    pointer(document.body, 'mouseout')
    await after(DELAY.hide * 2)
    expect(cardOf(wrapper).exists()).toBe(true)
  })
})
