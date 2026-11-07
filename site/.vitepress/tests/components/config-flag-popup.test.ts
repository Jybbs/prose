// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { Popup }           from '../../kit'
import { ConfigFlagPopup } from '../../theme/components/sandbox/ConfigFlagPopup'

describe('ConfigFlagPopup', () => {
  it('leaves focus where it is while the card shows', () => {
    const wrapper = mount(ConfigFlagPopup, { global: { stubs: { teleport: true } }, props: { rules: [] } })
    expect(wrapper.findComponent(Popup).props('passive')).toBe(true)
  })
})
