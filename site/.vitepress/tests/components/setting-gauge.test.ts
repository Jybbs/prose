// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { SettingGauge } from '../../theme/components/fixtures/SettingGauge'

const GAUGE = { default: 2, outputs: ['<pre>a</pre>', '<pre>b</pre>'], setting: 'max-shift', steps: [0, 0, 1, 1, 1] }

describe('SettingGauge', () => {
  it('steps the setting from the keyboard and reports the new value', async () => {
    const wrapper = mount(SettingGauge, { props: { gauge: GAUGE, modelValue: 2 } })
    const stop    = wrapper.get('.fixture-gauge-stop')
    await stop.trigger('focusin')
    await stop.trigger('keydown', { key: 'ArrowRight' })
    await stop.trigger('keydown', { key: 'End' })
    expect(wrapper.emitted('update:modelValue')).toStrictEqual([[3], [4]])
  })

  it('marks the zero, the default, and the last value under the track', () => {
    const nums = mount(SettingGauge, { props: { gauge: GAUGE, modelValue: 0 } }).findAll('.fixture-gauge-num')
    expect(nums.map(num => [num.text(), num.attributes('data-end')])).toStrictEqual([['0', 'start'], ['2', undefined], ['4', 'end']])
  })
})
