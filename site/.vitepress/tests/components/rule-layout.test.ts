// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import CompositionCards from '../../theme/components/rules/CompositionCards.vue'
import RuleLayout       from '../../theme/components/rules/RuleLayout.vue'

vi.mock('../../lib/rules/composition.data', () => ({
  data: {
    byRule : { 'align-equals': ['alpha_case', 'beta_case'] },
    cases  : [{ case: 'alpha_case' }, { case: 'beta_case' }]
  }
}))

vi.mock('../../lib/rules/rule-fixtures.data', () => ({
  data: { align_equals: { canonical: 'basic_run', examples: [] } }
}))

const { frontmatter } = vi.hoisted(() => {
  const value: Record<string, unknown> = {}
  return { frontmatter: { value } }
})

vi.mock('vitepress', async importOriginal => ({
  ...(await importOriginal<typeof import('vitepress')>()),
  useData: () => ({ frontmatter })
}))

const STUBS = {
  global: {
    stubs: {
      CompositionCards : true,
      DocHeading       : true,
      Fixture          : true,
      RuleCardList     : true,
      RuleConfigTable  : true,
      RuleFixtures     : true
    }
  }
}

const composes = (): boolean =>
  mount(RuleLayout, { ...STUBS, props: { rule: 'align_equals' } })
    .findComponent(CompositionCards)
    .exists()

describe('RuleLayout', () => {
  afterEach(() => { delete frontmatter.value.inlineFixtures })

  it('renders the composition cards while the page embeds only some of its cases inline', () => {
    frontmatter.value.inlineFixtures = ['composition/alpha_case']
    expect(composes()).toBe(true)
  })

  it('renders no composition cards once the page embeds every case its rule takes part in', () => {
    frontmatter.value.inlineFixtures = ['composition/alpha_case', 'composition/beta_case']
    expect(composes()).toBe(false)
  })
})
