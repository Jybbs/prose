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

// Waits past the frames the machine focuses and dismisses on.
const nextFrames = () => new Promise(resolve => { setTimeout(resolve, 60) })

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
  const mountPopup = (theme: string, props: { anchor?: HTMLElement, open?: boolean, passive?: boolean } = {}) => mount(Popup, {
    attachTo : document.body,
    props    : { label: 'A note', theme, ...props },
    slots    : {
      default : () => h('a', { class: 'more', href: '/more' }, 'Read more'),
      trigger : scope => h('span', { ...scope.props, class: 'anchor', tabindex: 0 })
    }
  })

  const focused = async (selector: string) => {
    await vi.waitFor(() => { expect(document.activeElement?.matches(selector)).toBe(true) })
  }

  it('teleports nothing until it first opens', () => {
    const wrapper = mountPopup('closed')
    expect(document.querySelector('[data-popup="closed"]')).toBeNull()
    wrapper.unmount()
  })

  it('shows and hides its content through the open prop, keeping its positioner', async () => {
    const wrapper = mountPopup('driven', { open: true })
    expect(popupsIn('driven')).toHaveLength(1)
    expect(popupsIn('driven')[0].textContent).toContain('Read more')

    await wrapper.setProps({ open: false })
    expect(popupsIn('driven')).toHaveLength(0)
    expect(document.querySelector('[data-popup="driven"]')).not.toBeNull()
    wrapper.unmount()
  })

  it('opens from a click on its trigger, naming the dialog', async () => {
    const wrapper = mountPopup('clicked')
    await wrapper.get('.anchor').trigger('click')
    const [content] = popupsIn('clicked')
    expect(content.getAttribute('role')).toBe('dialog')
    expect(content.getAttribute('aria-label')).toBe('A note')
    wrapper.unmount()
  })

  it('stays closed under a pointer on its trigger', async () => {
    const wrapper = mountPopup('hovered')
    await wrapper.get('.anchor').trigger('pointerenter', { pointerType: 'mouse' })
    await wrapper.get('.anchor').trigger('pointermove', { pointerType: 'mouse' })
    await wrapper.get('.anchor').trigger('focus')
    expect(document.querySelector('[data-popup="hovered"]')).toBeNull()
    wrapper.unmount()
  })

  it.each(['Enter', ' '])('opens from %j and moves focus to the link inside', async key => {
    const wrapper = mountPopup('keyed')
    await wrapper.get('.anchor').trigger('keydown', { key })
    expect(popupsIn('keyed')).toHaveLength(1)
    await focused('[data-popup="keyed"] .more')
    wrapper.unmount()
  })

  it('closes on Escape and returns focus to the trigger', async () => {
    const wrapper = mountPopup('escaped')
    await wrapper.get('.anchor').trigger('click')
    await focused('[data-popup="escaped"] .more')
    await vi.waitFor(() => {
      document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' }))
      expect(popupsIn('escaped')).toHaveLength(0)
    })
    await focused('.anchor')
    wrapper.unmount()
  })

  it('leaves focus alone when passive, opening and closing', async () => {
    const outside = document.createElement('button')
    document.body.append(outside)
    outside.focus()
    const wrapper = mountPopup('passive', { anchor: outside, open: false, passive: true })
    await wrapper.setProps({ open: true })
    await nextFrames()
    expect(popupsIn('passive')).toHaveLength(1)
    expect(document.activeElement).toBe(outside)
    outside.blur()
    await wrapper.setProps({ open: false })
    await nextFrames()
    expect(document.activeElement).toBe(document.body)
    wrapper.unmount()
    outside.remove()
  })

  it('moves focus into a card a caller opens, and back to its anchor', async () => {
    const flag = document.createElement('button')
    document.body.append(flag)
    const wrapper = mountPopup('delegated', { anchor: flag, open: false })
    await wrapper.setProps({ open: true })
    await focused('[data-popup="delegated"] .more')
    await wrapper.setProps({ open: false })
    await vi.waitFor(() => { expect(document.activeElement).toBe(flag) })
    wrapper.unmount()
    flag.remove()
  })

  it('reports a dismissal to a caller holding its open state, staying open', async () => {
    const wrapper = mountPopup('dismissed', { anchor: document.body, open: true })
    await nextFrames()
    document.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' }))
    await vi.waitFor(() => { expect(wrapper.emitted('dismiss')).toHaveLength(1) })
    expect(popupsIn('dismissed')).toHaveLength(1)
    wrapper.unmount()
  })
})
