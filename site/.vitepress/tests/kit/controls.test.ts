// @vitest-environment happy-dom
import { flushPromises, mount }    from '@vue/test-utils'
import { defineComponent, h, ref } from 'vue'

import { Disclosure, Listbox, Popup, Slider, Tabs } from '../../kit'
import type { DisclosureScope }                     from '../../kit/controls/Disclosure'
import type { TabsScope }                           from '../../kit/controls/Tabs'

const OPTIONS = [{ id: 'one' }, { id: 'two' }] as const

const TabsHost = defineComponent(() => {
  const selected = ref('one')
  return () => h(Tabs<string>, {
    label                 : 'Views',
    modelValue            : selected.value,
    'onUpdate:modelValue' : value => { selected.value = value }
  }, {
    default: (scope: TabsScope<string>) => [
      h('div', { ...scope.list, class: 'list' }, OPTIONS.map(({ id }) => h('button', { ...scope.trigger(id), class: 'tab' }, id))),
      h('div', { ...scope.content(selected.value), class: 'panel' }, selected.value)
    ]
  })
})

const DisclosureHost = defineComponent(() => {
  const open = ref(false)
  return () => h(Disclosure, { open: open.value, 'onUpdate:open': value => { open.value = value } }, {
    default: (scope: DisclosureScope) => [
      h('button', { ...scope.trigger, class: 'trigger' }),
      h('div', { ...scope.content, class: 'body' })
    ]
  })
})

const mountListbox = () => mount(Listbox<{ id: string }>, {
  props : { label: 'Outcome', modelValue: 'one', options: OPTIONS },
  slots : {
    default: scope => [
      h('button', { ...scope.trigger, class: 'trigger' }),
      h('div', scope.positioner, h('ul', { ...scope.content, class: 'list' }, OPTIONS.map(option =>
        h('li', { ...scope.item(option), class: 'option' }, option.id))))
    ]
  }
})

const popupsIn = (theme: string) => document.querySelectorAll(`[data-popup="${theme}"] .z-10000`)

describe('Tabs', () => {
  it.each(['click', 'focus', 'mouseenter'])('selects a tab on %s', async event => {
    const wrapper         = mount(TabsHost)
    const [first, second] = wrapper.findAll('.tab')
    await second.trigger(event)
    expect(second.attributes('aria-selected')).toBe('true')
    expect(first.attributes('aria-selected')).toBe('false')
    expect(wrapper.get('.panel').text()).toBe('two')
  })

  it('names the list through its label', () => {
    expect(mount(TabsHost).get('.list').attributes('aria-label')).toBe('Views')
  })
})

describe('Disclosure', () => {
  it('opens and closes its body from the trigger', async () => {
    const wrapper = mount(DisclosureHost, { attachTo: document.body })
    const trigger = wrapper.get('.trigger')
    expect(trigger.attributes('aria-expanded')).toBe('false')
    expect(wrapper.get('.body').attributes('hidden')).toBeDefined()

    await trigger.trigger('click')
    await flushPromises()
    expect(trigger.attributes('aria-expanded')).toBe('true')
    expect(wrapper.get('.body').attributes('hidden')).toBeUndefined()

    await trigger.trigger('click')
    await vi.waitFor(() => { expect(trigger.attributes('aria-expanded')).toBe('false') })
    expect(wrapper.get('.body').attributes('hidden')).toBeDefined()
    wrapper.unmount()
  })
})

describe('Slider', () => {
  const mountSlider = (attachTo?: HTMLElement) => mount(Slider, {
    attachTo,
    props : { control: 'track', label: 'Code', max: 180, min: 30, modelValue: 88 },
    slots : { default: scope => h('button', { ...scope.thumb, class: 'thumb' }) }
  })

  it('names the thumb through its label and leaves its placing to the slot', () => {
    const thumb = mountSlider().get('.thumb')
    expect(thumb.attributes('aria-label')).toBe('Code')
    expect(thumb.attributes('aria-valuenow')).toBe('88')
    expect(thumb.attributes('style')).toBeUndefined()
  })

  it.each([
    ['ArrowRight', 89],
    ['ArrowLeft',  87],
    ['Home',       30],
    ['End',        180]
  ])('reports %s as %i through change and then commit', async (key, value) => {
    const wrapper = mountSlider(document.body)
    await wrapper.get('.thumb').trigger('focusin')
    await wrapper.get('.thumb').trigger('keydown', { key })
    await flushPromises()
    expect(wrapper.emitted('change')).toStrictEqual([[value]])
    expect(wrapper.emitted('commit')).toHaveLength(1)
    wrapper.unmount()
  })

  it('reports the point a drag reaches on the track, then commits on release', async () => {
    const track = document.createElement('div')
    track.id                    = 'track'
    track.getBoundingClientRect = () => new DOMRect(0, 0, 150, 10)
    document.body.append(track)

    const wrapper = mountSlider(track)
    await wrapper.get('.thumb').trigger('pointerdown', { button: 0, clientX: 0, clientY: 0, pointerId: 1 })
    document.dispatchEvent(new PointerEvent('pointermove', { button: 0, buttons: 1, clientX: 120, clientY: 5, pointerId: 1 }))
    await flushPromises()
    expect(wrapper.emitted('change')!.at(-1)).toStrictEqual([150])
    expect(wrapper.emitted('commit')).toBeUndefined()

    document.dispatchEvent(new PointerEvent('pointerup', { pointerId: 1 }))
    await flushPromises()
    expect(wrapper.emitted('commit')).toHaveLength(1)

    wrapper.unmount()
    track.remove()
  })
})

describe('Listbox', () => {
  it('names the trigger and the list through its label', () => {
    const wrapper = mountListbox()
    expect(wrapper.get('.trigger').attributes('aria-label')).toBe('Outcome')
    expect(wrapper.get('.list').attributes('aria-label')).toBe('Outcome')
  })

  it('reports the option picked once the trigger has opened the list', async () => {
    const wrapper = mountListbox()
    await wrapper.get('.trigger').trigger('click')
    expect(wrapper.get('.trigger').attributes('aria-expanded')).toBe('true')
    await wrapper.findAll('.option')[1].trigger('click')
    expect(wrapper.emitted('update:modelValue')).toStrictEqual([['two']])
    expect(wrapper.get('.trigger').attributes('aria-expanded')).toBe('false')
  })

  it('holds its value on an option click while the list is closed', async () => {
    const wrapper = mountListbox()
    await wrapper.findAll('.option')[1].trigger('click')
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  })
})

describe('Popup', () => {
  const mountPopup = (theme: string, props: { open?: boolean, openDelay?: number } = {}) => mount(Popup, {
    attachTo : document.body,
    props    : { theme, ...props },
    slots    : {
      default : () => 'content',
      trigger : scope => h('a', { ...scope.props, class: 'anchor' })
    }
  })

  it('teleports nothing until it first opens', () => {
    const wrapper = mountPopup('closed')
    expect(document.querySelector('[data-popup="closed"]')).toBeNull()
    wrapper.unmount()
  })

  it('shows and hides its content through the open prop, keeping its positioner', async () => {
    const wrapper = mountPopup('driven', { open: true })
    expect(popupsIn('driven')).toHaveLength(1)
    expect(popupsIn('driven')[0].textContent).toContain('content')

    await wrapper.setProps({ open: false })
    expect(popupsIn('driven')).toHaveLength(0)
    expect(document.querySelector('[data-popup="driven"]')).not.toBeNull()
    wrapper.unmount()
  })

  it('opens from a pointer on its trigger', async () => {
    const wrapper = mountPopup('pointed', { openDelay: 0 })
    await wrapper.get('.anchor').trigger('pointerenter', { pointerType: 'mouse' })
    await vi.waitFor(() => { expect(popupsIn('pointed')).toHaveLength(1) })
    wrapper.unmount()
  })
})
