// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

const { browser, route } = vi.hoisted(() => ({
  browser : { value: true },
  route   : { value: { relativePath: 'rules/alignment/align-equals.md' } }
}))

vi.mock('vitepress', () => ({
  get inBrowser() { return browser.value },
  useData: () => ({ page: route })
}))
vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ slug: 'align-equals' }]))

import * as composables from '../../lib/composables/route'
import { mountSetup }   from '../dom'

describe('useCurrentRule', () => {
  it('resolves the rule for the current route slug', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    expect(mountSetup(composables.useCurrentRule).value?.slug).toBe('align-equals')
  })

  it.each([
    ['off a rule page',                  'reference/cli.md'],
    ['on a rules index route',           'rules/index.md'],
    ['on a route whose slug is unknown', 'rules/alignment/not-a-rule.md']
  ])('returns null %s', (_name, relativePath) => {
    route.value = { relativePath }
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

describe('useFamilyDataset', () => {
  beforeEach(() => { browser.value = true })

  it('mirrors the family of a rule route onto the body element', () => {
    route.value = { relativePath: 'rules/alignment/align-equals.md' }
    mountSetup(composables.useFamilyDataset)
    expect(document.body.dataset.family).toBe('alignment')
  })

  it('clears the attribute off the rules tree', () => {
    document.body.dataset.family = 'alignment'
    route.value = { relativePath: 'usage/index.md' }
    mountSetup(composables.useFamilyDataset)
    expect(document.body.dataset.family).toBeUndefined()
  })

  it('leaves the attribute alone when there is no document', () => {
    document.body.dataset.family = 'alignment'
    browser.value = false
    route.value = { relativePath: 'rules/ordering/alphabetize-siblings.md' }
    mountSetup(composables.useFamilyDataset)
    expect(document.body.dataset.family).toBe('alignment')
  })
})
