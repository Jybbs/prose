// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { nextTick } from 'vue'

import CompositionCards from '../../theme/components/rules/CompositionCards.vue'

vi.mock('../../lib/rules/composition.data', () => ({
  data: {
    byRule: {
      'align-equals'     : ['alpha_case', 'beta_case'],
      'space-statements' : ['alpha_case'],
      'wrap-docstrings'  : ['gamma_case']
    },
    cases: [
      { case: 'alpha_case', configToml: '', rules: ['align-equals', 'space-statements'],
        source: 'a = 1\n', titleHtml: 'Alpha Case' },
      { case: 'beta_case', configToml: '', rules: ['align-equals'],
        source: 'b = 2\n', titleHtml: 'Beta Case' },
      { case: 'gamma_case', configToml: '', rules: ['wrap-docstrings'],
        source: 'c = 3\n', titleHtml: 'Gamma <code>c = 3</code> Case' }
    ]
  }
}))

const { frontmatter } = vi.hoisted(() => {
  const fixture = {
    changesSource : true,
    hasFindings   : false,
    hasToggle     : true,
    inputHtml     : '',
    outputHtml    : ''
  }
  return {
    frontmatter: {
      value: {
        fixtures: {
          'composition/alpha_case' : fixture,
          'composition/beta_case'  : fixture,
          'composition/gamma_case' : fixture
        }
      }
    }
  }
})

vi.mock('vitepress', async importOriginal => ({
  ...(await importOriginal<typeof import('vitepress')>()),
  useData: () => ({ frontmatter })
}))

vi.mock('../../lib/rules/rules.data', () => ({ data: { bySlug: {} } }))

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
  afterEach(() => { window.location.hash = '' })

  it('renders every previewable case where no rule narrows the set', () => {
    expect(titles()).toEqual(['Alpha Case', 'Beta Case', 'Gamma c = 3 Case'])
  })

  it('renders a backticked title as a code span', () => {
    expect(render().get('#gamma_case .fixture-card-title').html()).toContain('<code>c = 3</code>')
  })

  it('renders only the cases the named rule takes part in', () => {
    expect(titles('space_statements')).toEqual(['Alpha Case'])
  })

  it('leaves each folio empty so the fixture counter numbers it in place', () => {
    const nums = render('wrap_docstrings').findAll('.fixture-card-num').map(node => node.text())
    expect(nums).toEqual([''])
  })

  it('renders an empty list for a rule no previewable case activates', () => {
    expect(titles('align_colons')).toEqual([])
  })

  it('renders the before-and-after pair only for the open card', async () => {
    const wrapper = render()
    expect(wrapper.findAll('.fixture-pair-doc')).toHaveLength(0)
    await wrapper.get('#beta_case').trigger('click')
    expect(wrapper.findAll('.fixture-pair-doc')).toHaveLength(1)
  })

  it('opens the card the address-bar fragment names', async () => {
    window.location.hash = '#gamma_case'
    const wrapper = render()
    await nextTick()
    expect(wrapper.get('#gamma_case').classes()).toContain('is-open')
  })

  it('opens no card for a fragment naming none of them', async () => {
    window.location.hash = '#not_a_case'
    const wrapper = render()
    await nextTick()
    expect(wrapper.findAll('.fixture-card.is-open')).toHaveLength(0)
  })
})
