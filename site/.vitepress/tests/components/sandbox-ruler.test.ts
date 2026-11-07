// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import { SandboxRuler }    from '../../theme/components/sandbox/SandboxRuler'
import type { LengthKnob } from '../../lib/sandbox/config-schema.data'

const KNOBS: readonly LengthKnob[] = [
  { default: 88, key: 'code-line-length',      label: 'Code' },
  { default: 76, key: 'docstring-line-length', label: 'Docstring' }
]

const mountRuler = (values: Record<string, number> = {}) =>
  mount(SandboxRuler, {
    props: {
      lengths : KNOBS,
      lengthOf : (key: string) => values[key] ?? 88
    }
  })

describe('SandboxRuler', () => {
  it('marks a stop moved from its default and the stop a press holds', async () => {
    const wrapper = mountRuler({ 'code-line-length': 90, 'docstring-line-length': 76 })
    const [moved, resting] = wrapper.findAll('.sandbox-ruler-stop')
    expect(moved.attributes('data-changed')).toBe('true')
    expect(resting.attributes('data-changed')).toBeUndefined()
    moved.element.setPointerCapture = vi.fn<(pointerId: number) => void>()
    await moved.trigger('pointerdown', { button: 0, pointerId: 1 })
    expect(moved.attributes('data-dragging')).toBe('true')
    expect(resting.attributes('data-dragging')).toBeUndefined()
  })

  it('steps a stop by one on an arrow key and to the large step with shift', async () => {
    const wrapper = mountRuler()
    const stop    = wrapper.get('.sandbox-ruler-stop')
    await stop.trigger('focusin')
    await stop.trigger('keydown', { key: 'ArrowRight' })
    await flushPromises()
    await stop.trigger('keydown', { key: 'ArrowLeft', shiftKey: true })
    await flushPromises()
    expect(wrapper.emitted('setLength')).toStrictEqual([
      ['code-line-length', 89],
      ['code-line-length', 80]
    ])
  })

  it('jumps to the rail ends on Home and End', async () => {
    const wrapper = mountRuler()
    const stop    = wrapper.get('.sandbox-ruler-stop')
    await stop.trigger('focusin')
    await stop.trigger('keydown', { key: 'Home' })
    await flushPromises()
    await stop.trigger('keydown', { key: 'End' })
    await flushPromises()
    expect(wrapper.emitted('setLength')).toStrictEqual([
      ['code-line-length', 30],
      ['code-line-length', 180]
    ])
  })

  it('emits nothing for a step at the rail', async () => {
    const wrapper = mountRuler({ 'code-line-length': 180 })
    const stop    = wrapper.get('.sandbox-ruler-stop')
    await stop.trigger('focusin')
    await stop.trigger('keydown', { key: 'ArrowUp' })
    await flushPromises()
    expect(wrapper.emitted('setLength')).toBeUndefined()
  })

  it('leaves an unmapped key to the page', async () => {
    const wrapper = mountRuler()
    await wrapper.get('.sandbox-ruler-stop').trigger('keydown', { key: 'Tab' })
    expect(wrapper.emitted('setLength')).toBeUndefined()
  })

  it('commits a double-click edit through Enter, clamped to the rail', async () => {
    const wrapper = mountRuler()
    await wrapper.get('.sandbox-ruler-chip').trigger('dblclick')
    const input = wrapper.get('input.sandbox-ruler-chip-input')
    await input.setValue('300')
    await input.trigger('keydown.enter')
    expect(wrapper.emitted('setLength')).toStrictEqual([['code-line-length', 180]])
    expect(wrapper.find('input.sandbox-ruler-chip-input').exists()).toBe(false)
  })

  it('abandons an edit on Escape without emitting', async () => {
    const wrapper = mountRuler()
    await wrapper.get('.sandbox-ruler-chip').trigger('dblclick')
    await wrapper.get('input.sandbox-ruler-chip-input').trigger('keydown.esc')
    expect(wrapper.emitted('setLength')).toBeUndefined()
    expect(wrapper.find('input.sandbox-ruler-chip-input').exists()).toBe(false)
  })

  it('holds a stop inside the slop, previews a drag past it, and commits on release', async () => {
    vi.stubGlobal('ResizeObserver', class {
      constructor(private readonly report: ResizeObserverCallback) {}
      disconnect() {}
      observe(target: Element) { this.report([{ contentRect: { height: 10, width: 150 }, target } as unknown as ResizeObserverEntry], this as unknown as ResizeObserver) }
      unobserve() {}
    })
    const wrapper = mount(SandboxRuler, { attachTo: document.body, props: { lengths: KNOBS, lengthOf: () => 88 } })
    wrapper.get('.sandbox-ruler').element.getBoundingClientRect = () => new DOMRect(0, 0, 150, 10)
    const move = (clientX: number) => document.dispatchEvent(new PointerEvent('pointermove', { button: 0, buttons: 1, clientX, clientY: 5, pointerId: 1 }))

    await wrapper.get('.sandbox-ruler-stop').trigger('pointerdown', { button: 0, clientX: 0, clientY: 5, pointerId: 1 })
    move(60)
    await flushPromises()
    expect(wrapper.emitted('preview')).toBeUndefined()

    move(120)
    await flushPromises()
    expect(wrapper.emitted('preview')!.at(-1)).toStrictEqual(['code-line-length', 150])
    expect(wrapper.emitted('setLength')).toBeUndefined()

    document.dispatchEvent(new PointerEvent('pointerup', { pointerId: 1 }))
    await flushPromises()
    expect(wrapper.emitted('setLength')).toStrictEqual([['code-line-length', 150]])
    wrapper.unmount()
    vi.unstubAllGlobals()
  })

  it('drops the slider bindings from a stop under edit', async () => {
    const wrapper = mountRuler()
    expect(wrapper.get('.sandbox-ruler-stop').attributes('role')).toBe('slider')
    await wrapper.get('.sandbox-ruler-chip').trigger('dblclick')
    expect(wrapper.get('.sandbox-ruler-stop').attributes('role')).toBeUndefined()
  })
})
