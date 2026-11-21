// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { Switch } from '../../../theme/components/base/Switch'

const props = { checked: false, label: 'Wordmark' }

describe('Switch', () => {
  it.each([true, false])('names its state as aria-checked %s', checked => {
    expect(mount(Switch, { props: { ...props, checked } }).get('button').attributes()).toMatchObject({
      'aria-checked' : String(checked),
      'aria-label'   : 'Wordmark',
      'role'         : 'switch'
    })
  })

  it('emits toggle on each press', async () => {
    const wrapper = mount(Switch, { props })
    await wrapper.get('button').trigger('click')
    await wrapper.get('button').trigger('click')
    expect(wrapper.emitted('toggle')).toHaveLength(2)
  })

  it('adds the classes a parent passes to its own', () => {
    const button = mount(Switch, { attrs: { class: 'cursor-pointer' }, props }).get('button')
    expect(button.classes()).toStrictEqual(expect.arrayContaining(['group/switch', 'cursor-pointer']))
  })
})
