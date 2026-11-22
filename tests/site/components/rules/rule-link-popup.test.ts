// @vitest-environment happy-dom
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick }               from 'vue'

import { Popup }         from '../../../../site/.vitepress/kit'
import { RuleLinkPopup } from '../../../../site/.vitepress/theme/components/rules/RuleLinkPopup'

const DELAY = { hide: 120, show: 80 }

vi.mock('../../../../site/.vitepress/lib/rules/rules.data', async () =>
  (await import('../../rules-data-stub')).rulesDataStub([{ family: 'lint', slug: 'line-overflow' }]))

const stubs = { teleport: true, RuleCard: { template: '<div class="rule-card"><span class="rule-card-slug" /></div>' } }

const after = async (ms: number): Promise<void> => {
  vi.advanceTimersByTime(ms)
  await nextTick()
}

const cardOf = (wrapper: VueWrapper) => wrapper.find('.rule-card')

const escape = async (): Promise<void> => {
  await vi.runAllTimersAsync()
  document.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' }))
  await vi.runAllTimersAsync()
}

// Sends a pointer event, its `relatedTarget` null where the pointer leaves the window.
const pointer = (
  target  : Element,
  type    : 'mouseout' | 'mouseover' | 'pointermove',
  related : Element | null = document.body
): void => {
  target.dispatchEvent(new MouseEvent(type, { bubbles: true, relatedTarget: related }))
}

// Mounts the popup beside one rule link and opens its card through the link.
async function opened(): Promise<{ link: Element, wrapper: VueWrapper }> {
  document.body.innerHTML = '<a class="rule-link" data-rule="line-overflow" href="#">'
                          + '<span class="badge"></span>Line Overflow</a><button></button>'
  const link    = document.querySelector('a')!
  const wrapper = mount(RuleLinkPopup, { attachTo: document.body, global: { stubs } })
  pointer(link, 'mouseover')
  await after(DELAY.show)
  return { link, wrapper }
}

describe('RuleLinkPopup', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('closes the card on Escape and leaves focus where it is', async () => {
    const { wrapper } = await opened()
    document.querySelector('button')!.focus()
    await escape()
    expect(cardOf(wrapper).exists()).toBe(false)
    expect(document.activeElement).toBe(document.querySelector('button'))
  })

  it('opens nothing as the pointer crosses the link Escape closed it on until it moves off it', async () => {
    const { link, wrapper } = await opened()
    await escape()
    pointer(link.querySelector('.badge')!, 'mouseover')
    pointer(link.querySelector('.badge')!, 'pointermove')
    pointer(link, 'mouseover')
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(false)
    pointer(document.body, 'pointermove')
    pointer(link, 'mouseover')
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(true)
  })

  it('opens the card again once the pointer leaves the window off its link and returns', async () => {
    const { link, wrapper } = await opened()
    await escape()
    pointer(link, 'mouseout', null)
    pointer(link, 'pointermove')
    pointer(link, 'mouseover')
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(true)
  })

  it('opens nothing as the page shifts another link under a still pointer and back', async () => {
    const { link, wrapper } = await opened()
    await escape()
    const other = document.createElement('a')
    other.dataset.rule = 'line-overflow'
    document.body.append(other)
    pointer(link, 'mouseout')
    pointer(other, 'mouseover')
    pointer(other, 'mouseout')
    pointer(link, 'mouseover')
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(false)
  })

  it('opens again on the next focus of the link Escape closed it on', async () => {
    const { link, wrapper } = await opened()
    await escape()
    link.dispatchEvent(new FocusEvent('focusin', { bubbles: true }))
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(true)
  })

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

  it('opens no card for a pointer that leaves its link inside the show delay', async () => {
    document.body.innerHTML = '<a class="rule-link" data-rule="line-overflow"></a>'
    const link    = document.querySelector('a')!
    const wrapper = mount(RuleLinkPopup, { attachTo: document.body, global: { stubs } })
    pointer(link, 'mouseover')
    pointer(link, 'mouseout')
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(false)
  })

  it('ignores a pointer moving over the rest of the page', async () => {
    const { wrapper } = await opened()
    pointer(document.body, 'mouseover')
    pointer(document.body, 'mouseout')
    await after(DELAY.hide * 2)
    expect(cardOf(wrapper).exists()).toBe(true)
  })

  it('opens the card once focus reaches its link and closes it once focus leaves', async () => {
    document.body.innerHTML = '<a class="rule-link" data-rule="line-overflow"></a>'
    const link    = document.querySelector('a')!
    const wrapper = mount(RuleLinkPopup, { attachTo: document.body, global: { stubs } })
    link.dispatchEvent(new FocusEvent('focusin', { bubbles: true }))
    await after(DELAY.show)
    expect(cardOf(wrapper).exists()).toBe(true)
    link.dispatchEvent(new FocusEvent('focusout', { bubbles: true }))
    await after(DELAY.hide)
    expect(cardOf(wrapper).exists()).toBe(false)
  })

  it('leaves focus where it is while the card shows', async () => {
    const { wrapper } = await opened()
    expect(wrapper.findComponent(Popup).props('passive')).toBe(true)
  })
})
