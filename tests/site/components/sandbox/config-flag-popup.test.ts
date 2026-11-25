// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { nextTick } from 'vue'

import { Popup }                      from '../../../../site/.vitepress/kit'
import { ConfigFlagPopup }            from '../../../../site/.vitepress/theme/components/sandbox/ConfigFlagPopup'
import type { ConfigFlagPopupHandle } from '../../../../site/.vitepress/theme/components/sandbox/ConfigFlagPopup'

const MESSAGE = 'warning: unknown key `rules.align-equals.max-shfit` in [tool.prose]'

const escape = async (): Promise<void> => {
  document.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' }))
  await vi.runAllTimersAsync()
}

// Mounts the popup beside one flag holding a token span and opens its card
// through a token inside the flag.
async function opened() {
  vi.useFakeTimers()
  document.body.innerHTML = `<span class="lint-flag" data-message="${MESSAGE}">`
                          + '<span class="token"></span></span><span class="code"></span>'
  const flag    = document.querySelector<HTMLElement>('.lint-flag')!
  const wrapper = mount(ConfigFlagPopup, {
    attachTo : document.body,
    global   : { stubs: { teleport: true } },
    props    : { rules: [] }
  })
  const popup  = wrapper.vm as unknown as ConfigFlagPopupHandle
  const isOpen = () => wrapper.findComponent(Popup).props('open')
  popup.show({ target: flag.querySelector('.token') } as unknown as Event)
  await vi.runAllTimersAsync()
  return { flag, isOpen, popup }
}

describe('ConfigFlagPopup', () => {
  afterEach(() => { vi.useRealTimers() })

  it('closes the card on Escape and leaves focus where it is', async () => {
    const { isOpen } = await opened()
    const before     = document.activeElement
    await escape()
    expect(isOpen()).toBe(false)
    expect(document.activeElement).toBe(before)
  })

  it('opens nothing as the pointer crosses the flag Escape closed it on until it leaves', async () => {
    const { flag, isOpen, popup } = await opened()
    await escape()
    popup.show({ target: flag.querySelector('.token') } as unknown as Event)
    flag.querySelector('.token')!.dispatchEvent(new MouseEvent('pointermove', { bubbles: true }))
    popup.show({ target: flag } as unknown as Event)
    await nextTick()
    expect(isOpen()).toBe(false)
    popup.hide()
    popup.show({ target: flag } as unknown as Event)
    await nextTick()
    expect(isOpen()).toBe(true)
  })

  it('opens the flag Escape closed it on once the pointer moves off it inside the pane', async () => {
    const { flag, isOpen, popup } = await opened()
    await escape()
    document.querySelector('.code')!.dispatchEvent(new MouseEvent('pointermove', { bubbles: true }))
    popup.show({ target: flag } as unknown as Event)
    await nextTick()
    expect(isOpen()).toBe(true)
  })

  it('holds nothing at an Escape pressed with the pointer on code text outside the flag', async () => {
    const { flag, isOpen, popup } = await opened()
    popup.show({ target: document.querySelector('.code') } as unknown as Event)
    await escape()
    popup.show({ target: flag } as unknown as Event)
    await nextTick()
    expect(isOpen()).toBe(true)
  })

  it('leaves focus where it is while the card shows', () => {
    const wrapper = mount(ConfigFlagPopup, { global: { stubs: { teleport: true } }, props: { rules: [] } })
    expect(wrapper.findComponent(Popup).props('passive')).toBe(true)
  })

  it('closes on the schedule of the first leave however many leaves follow', async () => {
    vi.useFakeTimers()
    const flag = document.createElement('span')
    flag.className       = 'lint-flag'
    flag.dataset.message = MESSAGE
    const wrapper = mount(ConfigFlagPopup, { global: { stubs: { teleport: true } }, props: { rules: [] } })
    const popup   = wrapper.vm as unknown as ConfigFlagPopupHandle
    popup.show({ target: flag } as unknown as Event)
    await nextTick()
    expect(wrapper.findComponent(Popup).props('open')).toBe(true)
    popup.hide()
    vi.advanceTimersByTime(100)
    popup.hide()
    vi.advanceTimersByTime(20)
    await nextTick()
    expect(wrapper.findComponent(Popup).props('open')).toBe(false)
  })
})
