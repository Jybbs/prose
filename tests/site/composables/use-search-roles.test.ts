// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent }      from 'vue'

import type { VueWrapper } from '@vue/test-utils'

import { useSearchRoles } from '../../../site/.vitepress/lib/composables/use-search-roles'

const row = (index: number, selected: boolean) =>
  `<li role="option" id="localsearch-item-${index}" aria-selected="${selected}">`
  + `<a class="result" href="/page-${index}" aria-label="Result ${index}">Result ${index}</a></li>`

const BOX = '<div class="VPLocalSearchBox" role="button" aria-expanded="true" aria-haspopup="listbox">'
  + '<kbd aria-label="Enter">Enter</kbd><ul class="results">' + row(0, true) + row(1, false) + '</ul></div>'

const settle = async () => {
  await flushPromises()
  await flushPromises()
}

const openBox = async (): Promise<HTMLElement> => {
  document.body.insertAdjacentHTML('beforeend', BOX)
  await settle()
  return document.querySelector<HTMLElement>('body > .VPLocalSearchBox')!
}

const links = (box: HTMLElement) => [...box.querySelectorAll<HTMLElement>('a.result')]

describe('useSearchRoles', () => {
  let wrapper : VueWrapper
  beforeEach(() => { wrapper = mount(defineComponent({ setup: () => { useSearchRoles(); return () => null } })) })
  afterEach(() => {
    wrapper.unmount()
    document.body.replaceChildren()
  })

  it('makes the box a dialog and each shortcut key an image', async () => {
    const box = await openBox()
    expect(box.getAttribute('role')).toBe('dialog')
    expect(box.hasAttribute('aria-expanded')).toBe(false)
    expect(box.hasAttribute('aria-haspopup')).toBe(false)
    expect(box.querySelector('kbd')!.getAttribute('role')).toBe('img')
  })

  it('moves the option role, the id, and the selection from each row to its link', async () => {
    const box = await openBox()
    for (const [index, link] of links(box).entries()) {
      const item = link.parentElement!
      expect(item.getAttribute('role')).toBe('none')
      expect(item.hasAttribute('id')).toBe(false)
      expect(item.hasAttribute('aria-selected')).toBe(false)
      expect(link.getAttribute('role')).toBe('option')
      expect(link.id).toBe(`localsearch-item-${index}`)
      expect(link.getAttribute('aria-selected')).toBe(String(index === 0))
      expect(link.hasAttribute('aria-label')).toBe(false)
    }
  })

  it('moves a selection written back to a row onto its link', async () => {
    const box             = await openBox()
    const [first, second] = links(box)
    first!.parentElement!.setAttribute('aria-selected', 'false')
    second!.parentElement!.setAttribute('aria-selected', 'true')
    await settle()
    expect(first!.getAttribute('aria-selected')).toBe('false')
    expect(second!.getAttribute('aria-selected')).toBe('true')
    expect(box.querySelectorAll('li[aria-selected]')).toHaveLength(0)
  })

  it('repairs a row rendered after the box opens', async () => {
    const box = await openBox()
    box.querySelector('.results')!.insertAdjacentHTML('beforeend', row(2, false))
    await settle()
    const link = links(box)[2]!
    expect(link.getAttribute('role')).toBe('option')
    expect(link.id).toBe('localsearch-item-2')
    expect(link.parentElement!.getAttribute('role')).toBe('none')
  })

  it('repairs a box opened again after it closes', async () => {
    const closed = await openBox()
    closed.remove()
    await settle()
    const box = await openBox()
    expect(box.getAttribute('role')).toBe('dialog')
    expect(links(box).map(link => link.getAttribute('role'))).toStrictEqual(['option', 'option'])
  })

  it('leaves the body alone while no box is open', async () => {
    document.body.insertAdjacentHTML('beforeend', '<div class="other" role="button"></div>')
    await settle()
    expect(document.querySelector('.other')!.getAttribute('role')).toBe('button')
  })
})
