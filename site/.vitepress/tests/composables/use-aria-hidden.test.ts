// @vitest-environment happy-dom
import { mount }                   from '@vue/test-utils'
import { defineComponent, h, ref } from 'vue'

import { provideAriaHidden, useAriaHidden, useHiddenTabindex } from '../../lib/composables/use-aria-hidden'
import { mountSetup } from '../dom'

function mountProviding<T>(source: Parameters<typeof provideAriaHidden>[0], read: () => T): T {
  let injected!: T
  const Child = defineComponent({
    setup: () => { injected = read(); return () => h('span') }
  })
  const Parent = defineComponent({
    setup: () => { provideAriaHidden(source); return () => h(Child) }
  })
  mount(Parent)
  return injected
}

describe('useAriaHidden', () => {
  it('reads false where no ancestor provides a value', () => {
    expect(mountSetup(useAriaHidden).value).toBe(false)
  })

  it('reads the value an ancestor provides', () => {
    expect(mountProviding(true, useAriaHidden).value).toBe(true)
  })

  it('tracks a provided source that changes', () => {
    const source = ref(false)
    const hidden = mountProviding(source, useAriaHidden)
    expect(hidden.value).toBe(false)
    source.value = true
    expect(hidden.value).toBe(true)
  })
})

describe('useHiddenTabindex', () => {
  it('leaves a node in the tab order outside a hidden subtree', () => {
    expect(mountSetup(useHiddenTabindex).value).toBeUndefined()
  })

  it('drops a node out of the tab order inside a hidden subtree', () => {
    expect(mountProviding(true, useHiddenTabindex).value).toBe(-1)
  })

  it('follows a provided source that changes', () => {
    const source   = ref(false)
    const tabindex = mountProviding(source, useHiddenTabindex)
    expect(tabindex.value).toBeUndefined()
    source.value = true
    expect(tabindex.value).toBe(-1)
  })
})
