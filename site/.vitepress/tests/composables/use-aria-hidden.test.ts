// @vitest-environment happy-dom
import { mount }                   from '@vue/test-utils'
import { defineComponent, h, ref } from 'vue'

import * as ariaHidden from '../../lib/composables/use-aria-hidden'
import { mountSetup }  from '../dom'

type HiddenSource = Parameters<typeof ariaHidden.provideAriaHidden>[0]

function mountProviding<T>(source: HiddenSource, read: () => T): T {
  let injected!: T
  const Child = defineComponent({
    setup: () => { injected = read(); return () => h('span') }
  })
  const Parent = defineComponent({
    setup: () => { ariaHidden.provideAriaHidden(source); return () => h(Child) }
  })
  mount(Parent)
  return injected
}

describe('ariaHidden.useAriaHidden', () => {
  it('reads false where no ancestor provides a value', () => {
    expect(mountSetup(ariaHidden.useAriaHidden).value).toBe(false)
  })

  it('reads the value an ancestor provides', () => {
    expect(mountProviding(true, ariaHidden.useAriaHidden).value).toBe(true)
  })

  it('tracks a provided source that changes', () => {
    const source = ref(false)
    const hidden = mountProviding(source, ariaHidden.useAriaHidden)
    expect(hidden.value).toBe(false)
    source.value = true
    expect(hidden.value).toBe(true)
  })
})

describe('ariaHidden.useHiddenTabindex', () => {
  it('leaves a node in the tab order outside a hidden subtree', () => {
    expect(mountSetup(ariaHidden.useHiddenTabindex).value).toBeUndefined()
  })

  it('drops a node out of the tab order inside a hidden subtree', () => {
    expect(mountProviding(true, ariaHidden.useHiddenTabindex).value).toBe(-1)
  })

  it('follows a provided source that changes', () => {
    const source   = ref(false)
    const tabindex = mountProviding(source, ariaHidden.useHiddenTabindex)
    expect(tabindex.value).toBeUndefined()
    source.value = true
    expect(tabindex.value).toBe(-1)
  })
})
