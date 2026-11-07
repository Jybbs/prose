// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { nextTick } from 'vue'

import { CompositionCards } from '../../theme/components/rules/CompositionCards'

vi.mock('../../lib/rules/composition.data', () => ({
  data: {
    byRule: {
      'align-equals'     : ['alpha_case', 'beta_case'],
      'space-statements' : ['alpha_case'],
      'wrap-docstrings'  : ['gamma_case']
    },
    cases: [
      { case: 'alpha_case', rules: ['align-equals', 'space-statements'] },
      { case: 'beta_case', rules: ['align-equals'] },
      { case: 'gamma_case', rules: ['wrap-docstrings'] }
    ]
  }
}))

const { frontmatter } = vi.hoisted(() => {
  const fixture = (titleHtml: string) => ({
    changesSource : true,
    hasFindings   : false,
    hasToggle     : true,
    inputHtml     : '',
    outputHtml    : '',
    sandboxSeed   : { configToml: '', source: '' },
    titleHtml
  })
  const value: Record<string, unknown> = {
    fixtures: {
      'composition/alpha_case' : fixture('Alpha Case'),
      'composition/beta_case'  : fixture('Beta Case'),
      'composition/gamma_case' : fixture('Gamma <code>c = 3</code> Case')
    }
  }
  return { frontmatter: { value } }
})

vi.mock('vitepress', async importOriginal => ({
  ...(await importOriginal<typeof import('vitepress')>()),
  useData: () => ({ frontmatter })
}))

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub())

const STUBS = {
  global: {
    stubs: {
      FixtureNoChange : true,
      FixtureToggle   : true,
      RuleSegmentChip : true
    }
  }
}

const render = (rule?: string) =>
  mount(CompositionCards, { ...STUBS, props: rule === undefined ? {} : { rule } })

const titles = (rule?: string): string[] =>
  render(rule).findAll('.fixture-card-title').map(node => node.text())

describe('CompositionCards', () => {
  afterEach(() => {
    window.location.hash = ''
    delete frontmatter.value.inlineFixtures
  })

  it('renders every previewable case where no rule narrows the set', () => {
    expect(titles()).toStrictEqual(['Alpha Case', 'Beta Case', 'Gamma c = 3 Case'])
  })

  it('renders a backticked title as a code span', () => {
    expect(render().get('#gamma_case .fixture-card-title').html()).toContain('<code>c = 3</code>')
  })

  it('renders only the cases the named rule takes part in', () => {
    expect(titles('space_statements')).toStrictEqual(['Alpha Case'])
  })

  it('leaves each folio empty so the fixture counter numbers it in place', () => {
    const nums = render('wrap_docstrings').findAll('.fixture-card-num').map(node => node.text())
    expect(nums).toStrictEqual([''])
  })

  it('leaves out a case its page embeds inline, which another page still renders', () => {
    frontmatter.value.inlineFixtures = ['composition/beta_case']
    expect(titles('align_equals')).toStrictEqual(['Alpha Case'])
    expect(titles()).toStrictEqual(['Alpha Case', 'Gamma c = 3 Case'])
    delete frontmatter.value.inlineFixtures
    expect(titles('align_equals')).toStrictEqual(['Alpha Case', 'Beta Case'])
  })

  it('renders an empty list for a rule no previewable case activates', () => {
    expect(titles('align_colons')).toStrictEqual([])
  })

  it('renders the before-and-after pair only for the open card', async () => {
    const wrapper = render()
    expect(wrapper.findAll('.fixture-pair-doc')).toHaveLength(0)
    await wrapper.get('#beta_case').trigger('click')
    expect(wrapper.findAll('.fixture-pair-doc')).toHaveLength(1)
  })

  it('opens a card from its summary button and keeps it open on a click in its body', async () => {
    const wrapper = render()
    await wrapper.get('#beta_case .fixture-card-summary').trigger('click')
    expect(wrapper.get('#beta_case').attributes('data-open')).toBe('true')
    await wrapper.get('#beta_case .fixture-card-body').trigger('click')
    expect(wrapper.get('#beta_case').attributes('data-open')).toBe('true')
  })

  it('shows the toggle bar on the open card alone', async () => {
    const wrapper = render()
    await wrapper.get('#beta_case').trigger('click')
    const bars = wrapper.findAll('.fixture-card .contents > div').map(bar => bar.attributes('style'))
    expect(bars).toStrictEqual(['display: none;', undefined, 'display: none;'])
  })

  it('opens the card the address-bar fragment names', async () => {
    window.location.hash = '#gamma_case'
    const wrapper = render()
    await nextTick()
    expect(wrapper.get('#gamma_case').attributes('data-open')).toBe('true')
  })

  it('opens no card for a fragment naming none of them', async () => {
    window.location.hash = '#not_a_case'
    const wrapper = render()
    await nextTick()
    expect(wrapper.findAll('.fixture-card[data-open]')).toHaveLength(0)
  })
})
