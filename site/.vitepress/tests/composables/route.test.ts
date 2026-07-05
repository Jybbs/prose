// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

const { route } = vi.hoisted(() => ({
  route: { value: { relativePath: 'rules/alignment/align-equals.md' } }
}))

vi.mock('vitepress', () => ({ useData: () => ({ page: route }) }))
vi.mock('../../data/rules.data', () => ({
  data: { bySlug: { 'align-equals': { name: 'Align Equals', slug: 'align-equals' } } }
}))

import * as composables from '../../lib/composables/route'
import { mountSetup }   from '../dom'

describe('useCurrentRule', () => {
  it('resolves the rule for the current route slug', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    expect(mountSetup(composables.useCurrentRule).value?.slug).toBe('align-equals')
  })

  it('returns null off a rule page', () => {
    route.value = { relativePath: 'reference/cli.md' }
    expect(mountSetup(composables.useCurrentRule).value).toBeNull()
  })

  it('returns null on a rules index route', () => {
    route.value = { relativePath: 'rules/index.md' }
    expect(mountSetup(composables.useCurrentRule).value).toBeNull()
  })
})

describe('provideCurrentRule', () => {
  it('shares the resolved rule with a descendant through inject', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    let injected: ReturnType<typeof composables.useCurrentRule> | undefined
    const Child  = defineComponent({ setup() { injected = composables.useCurrentRule(); return () => h('div') } })
    const Parent = defineComponent({ setup() { composables.provideCurrentRule(); return () => h(Child) } })
    mount(Parent)
    expect(injected?.value?.slug).toBe('align-equals')
  })
})

describe('useCurrentFamily', () => {
  it('reads the family segment of a rule route', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    expect(mountSetup(composables.useCurrentFamily).value).toBe('alignment')
  })

  it('returns null off the rules tree', () => {
    route.value = { relativePath: 'usage/index.md' }
    expect(mountSetup(composables.useCurrentFamily).value).toBeNull()
  })
})
