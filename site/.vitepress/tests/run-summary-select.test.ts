// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import RunSummarySelect from '../theme/components/reference/RunSummarySelect.vue'

const options = [
  { id: 'a', mono: 'AA' },
  { id: 'b', mono: 'BB' }
]

const mountSelect = (modelValue: string) => mount(RunSummarySelect, {
  props: { label: 'Verbosity', modelValue, options },
  global: {
    directives : { 'close-popper': {} },
    stubs      : { VDropdown: { template: '<div><slot /><slot name="popper" /></div>' } }
  }
})

describe('RunSummarySelect', () => {
  it('wires the label onto the trigger and the listbox', () => {
    const w = mountSelect('a')
    expect(w.get('.run-summary-select-trigger').attributes('aria-label')).toBe('Verbosity')
    expect(w.get('.run-summary-opts').attributes('aria-label')).toBe('Verbosity')
  })

  it('shows the selected option on the trigger', () => {
    expect(mountSelect('b').get('.run-summary-select-trigger').text()).toContain('BB')
  })

  it('falls back to the first option when the model matches none', () => {
    expect(mountSelect('zzz').get('.run-summary-select-trigger').text()).toContain('AA')
  })

  it('updates the model when an option is clicked', async () => {
    const w = mountSelect('a')
    await w.findAll('.run-summary-opt')[1].trigger('click')
    expect(w.emitted('update:modelValue')?.[0]).toEqual(['b'])
  })
})
