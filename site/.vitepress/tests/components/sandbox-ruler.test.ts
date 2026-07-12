// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import SandboxRuler         from '../../theme/components/sandbox/SandboxRuler.vue'
import type { LengthKnob }  from '../../lib/sandbox/config-schema.data'

const KNOBS: readonly LengthKnob[] = [
  { default: 88, key: 'code-line-length',      label: 'Code' },
  { default: 76, key: 'docstring-line-length', label: 'Docstring' }
]

const mountRuler = (values: Record<string, number> = {}) =>
  mount(SandboxRuler, {
    props: {
      lengths : KNOBS,
      valueOf : (key: string) => values[key] ?? 88
    }
  })

describe('SandboxRuler', () => {
  it('steps a stop by one on an arrow key and by ten with shift', async () => {
    const wrapper = mountRuler()
    const stop    = wrapper.get('.ruler-stop')
    await stop.trigger('keydown', { key: 'ArrowRight' })
    await stop.trigger('keydown', { key: 'ArrowLeft', shiftKey: true })
    expect(wrapper.emitted('setLength')).toEqual([
      ['code-line-length', 89],
      ['code-line-length', 78]
    ])
  })

  it('jumps to the rail ends on Home and End', async () => {
    const wrapper = mountRuler()
    const stop    = wrapper.get('.ruler-stop')
    await stop.trigger('keydown', { key: 'Home' })
    await stop.trigger('keydown', { key: 'End' })
    expect(wrapper.emitted('setLength')).toEqual([
      ['code-line-length', 30],
      ['code-line-length', 180]
    ])
  })

  it('clamps a step at the rail bounds', async () => {
    const wrapper = mountRuler({ 'code-line-length': 180 })
    const stop    = wrapper.get('.ruler-stop')
    await stop.trigger('keydown', { key: 'ArrowUp' })
    expect(wrapper.emitted('setLength')).toEqual([['code-line-length', 180]])
  })

  it('leaves an unmapped key to the page', async () => {
    const wrapper = mountRuler()
    await wrapper.get('.ruler-stop').trigger('keydown', { key: 'Tab' })
    expect(wrapper.emitted('setLength')).toBeUndefined()
  })

  it('commits a double-click edit through Enter, clamped to the rail', async () => {
    const wrapper = mountRuler()
    await wrapper.get('.ruler-chip').trigger('dblclick')
    const input = wrapper.get('input.ruler-chip-input')
    await input.setValue('300')
    await input.trigger('keydown.enter')
    expect(wrapper.emitted('setLength')).toEqual([['code-line-length', 180]])
    expect(wrapper.find('input.ruler-chip-input').exists()).toBe(false)
  })

  it('abandons an edit on Escape without emitting', async () => {
    const wrapper = mountRuler()
    await wrapper.get('.ruler-chip').trigger('dblclick')
    await wrapper.get('input.ruler-chip-input').trigger('keydown.esc')
    expect(wrapper.emitted('setLength')).toBeUndefined()
    expect(wrapper.find('input.ruler-chip-input').exists()).toBe(false)
  })
})
