// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

const { route } = vi.hoisted(() => ({
  route: { value: { relativePath: 'rules/alignment/align-equals.md' } }
}))

vi.mock('vitepress', () => ({ useData: () => ({ page: route }) }))
vi.mock('../data/rules.data', () => ({
  data: { bySlug: { 'align-equals': { name: 'Align Equals', slug: 'align-equals' } } }
}))

import { provideCurrentRule, useCurrentFamily, useCurrentRule } from '../lib/composables/route'

function capture<T>(fn: () => T): T {
  let value!: T
  mount(defineComponent({ setup() { value = fn(); return () => h('div') } }))
  return value
}

describe('useCurrentRule', () => {
  it('resolves the rule for the current route slug', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    expect(capture(useCurrentRule).value?.slug).toBe('align-equals')
  })

  it('returns null off a rule page', () => {
    route.value = { relativePath: 'reference/cli.md' }
    expect(capture(useCurrentRule).value).toBeNull()
  })

  it('returns null on a rules index route', () => {
    route.value = { relativePath: 'rules/index.md' }
    expect(capture(useCurrentRule).value).toBeNull()
  })
})

describe('provideCurrentRule', () => {
  it('shares the resolved rule with a descendant through inject', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    let injected: ReturnType<typeof useCurrentRule> | undefined
    const Child  = defineComponent({ setup() { injected = useCurrentRule(); return () => h('div') } })
    const Parent = defineComponent({ setup() { provideCurrentRule(); return () => h(Child) } })
    mount(Parent)
    expect(injected?.value?.slug).toBe('align-equals')
  })
})

describe('useCurrentFamily', () => {
  it('reads the family segment of a rule route', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    expect(capture(useCurrentFamily).value).toBe('alignment')
  })

  it('returns null off the rules tree', () => {
    route.value = { relativePath: 'usage/index.md' }
    expect(capture(useCurrentFamily).value).toBeNull()
  })
})
