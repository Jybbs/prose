// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { FixtureNoChange }   from '../../../../site/.vitepress/theme/components/fixtures/FixtureNoChange'
import { FixtureToggle }     from '../../../../site/.vitepress/theme/components/fixtures/FixtureToggle'
import { FixtureToggleSlot } from '../../../../site/.vitepress/theme/components/fixtures/FixtureToggleSlot'

const props = { hasToggle: true, modelValue: 'after' } as const

describe('FixtureToggleSlot', () => {
  it.each([
    { hasToggle: true,  shown: FixtureToggle },
    { hasToggle: false, shown: FixtureNoChange }
  ])('holds $shown.name where hasToggle is $hasToggle', ({ hasToggle, shown }) => {
    const wrapper = mount(FixtureToggleSlot, { props: { ...props, hasToggle } })
    expect(wrapper.findComponent(shown).exists()).toBe(true)
  })

  it('emits the tab the toggle chose to its parent', () => {
    const wrapper = mount(FixtureToggleSlot, { props })
    wrapper.getComponent(FixtureToggle).vm.$emit('update:modelValue', 'before')
    expect(wrapper.emitted('update:modelValue')).toStrictEqual([['before']])
  })

  it('keeps a press inside it from reaching the card', async () => {
    const outer   = vi.fn<(event: Event) => void>()
    const wrapper = mount(FixtureToggleSlot, { attachTo: document.body, props })
    document.body.addEventListener('click', outer)
    await wrapper.trigger('click')
    document.body.removeEventListener('click', outer)
    expect(outer).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})
