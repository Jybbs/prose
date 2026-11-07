// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { nextTick } from 'vue'

import { Popup }           from '../../kit'
import { ConfigFlagPopup } from '../../theme/components/sandbox/ConfigFlagPopup'

describe('ConfigFlagPopup', () => {
  afterEach(() => { vi.useRealTimers() })

  it('leaves focus where it is while the card shows', () => {
    const wrapper = mount(ConfigFlagPopup, { global: { stubs: { teleport: true } }, props: { rules: [] } })
    expect(wrapper.findComponent(Popup).props('passive')).toBe(true)
  })

  it('closes on the schedule of the first leave however many leaves follow', async () => {
    vi.useFakeTimers()
    const flag = document.createElement('span')
    flag.className       = 'lint-flag'
    flag.dataset.message = 'warning: unknown key `rules.align-equals.max-shfit` in [tool.prose]'
    const wrapper = mount(ConfigFlagPopup, { global: { stubs: { teleport: true } }, props: { rules: [] } })
    const popup   = wrapper.vm as unknown as { hide: () => void, show: (event: Event) => void }
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
