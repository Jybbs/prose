// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import FixtureCard  from '../../theme/components/fixtures/FixtureCard.vue'
import RuleFixtures from '../../theme/components/rules/RuleFixtures.vue'

vi.mock('../../lib/rules/rule-fixtures.data', () => ({
  data: {
    align_equals: {
      canonical : 'basic_run',
      examples  : [{ case: 'nested', titleHtml: 'Nested' }, { case: 'deep', titleHtml: 'Deep' }]
    }
  }
}))

const { frontmatter } = vi.hoisted(() => {
  const value: Record<string, unknown> = {}
  return { frontmatter: { value } }
})

vi.mock('vitepress', async importOriginal => ({
  ...(await importOriginal<typeof import('vitepress')>()),
  useData: () => ({ frontmatter })
}))

const STUBS = { global: { stubs: { FixtureCard: true } } }

const cases = (): unknown[] =>
  mount(RuleFixtures, { ...STUBS, props: { rule: 'align_equals' } })
    .findAllComponents(FixtureCard)
    .map(card => card.props('case'))

describe('RuleFixtures', () => {
  afterEach(() => { delete frontmatter.value.inlineFixtures })

  it('leaves out an example its page embeds inline, which another page still renders', () => {
    frontmatter.value.inlineFixtures = ['align_equals/nested']
    expect(cases()).toStrictEqual(['deep'])
    delete frontmatter.value.inlineFixtures
    expect(cases()).toStrictEqual(['nested', 'deep'])
  })
})
