// @vitest-environment happy-dom
import { mount }                   from '@vue/test-utils'
import { defineComponent, h, ref } from 'vue'

import { provideAriaHidden, useAriaHidden } from '../../lib/composables/use-aria-hidden'
import { mountSetup }                       from '../dom'

type Hidden = ReturnType<typeof useAriaHidden>

function mountProviding(source: Parameters<typeof provideAriaHidden>[0]): Hidden {
  let injected!: Hidden
  const Child = defineComponent({
    setup: () => { injected = useAriaHidden(); return () => h('span') }
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
    expect(mountProviding(true).value).toBe(true)
  })

  it('tracks a provided source that changes', () => {
    const source = ref(false)
    const hidden = mountProviding(source)
    expect(hidden.value).toBe(false)
    source.value = true
    expect(hidden.value).toBe(true)
  })
})
