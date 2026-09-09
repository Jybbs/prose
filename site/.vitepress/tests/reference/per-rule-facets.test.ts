// @vitest-environment happy-dom
import { mount }    from '@vue/test-utils'
import { nextTick } from 'vue'

import InlineProse          from '../../theme/components/base/InlineProse.vue'
import InlineRuleLink       from '../../theme/components/rules/InlineRuleLink.vue'
import PerRuleFacets        from '../../theme/components/reference/PerRuleFacets.vue'
import { expectAccessible } from '../axe'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub())

vi.mock('../../lib/reference/facets.data', () => ({
  data: [
    {
      badge : '',
      family: 'generic',
      label : 'Generic',
      rules : [
        { anchor: 'every-rule', rule: 'every rule', facets: [
          {
            anchor       : 'every-rule-enabled',
            default      : 'true',
            key          : 'enabled',
            meaningNodes : [{ kind: 'text', text: 'Toggle the rule.' }],
            type         : 'bool'
          }
        ] }
      ]
    },
    {
      badge : '🧺',
      family: 'layout',
      label : 'Layout',
      rules : [
        { anchor: 'reflow-calls', rule: 'reflow-calls', facets: [
          {
            anchor       : 'reflow-calls-max-args',
            default      : '3',
            key          : 'max-args',
            meaningNodes : [{ kind: 'text', text: 'Explode a call.' }],
            type         : 'positive int | false'
          }
        ] },
        { anchor: 'reflow-collections', rule: 'reflow-collections', facets: [
          {
            anchor       : 'reflow-collections-keep-multiline-literals',
            default      : 'true',
            key          : 'keep-multiline-literals',
            meaningNodes : [
              { kind: 'text', text: 'Join with ' },
              { kind: 'code', text: 'false' },
              { kind: 'text', text: '.' }
            ],
            type         : 'bool'
          },
          {
            anchor       : 'reflow-collections-max-atomics',
            default      : '8',
            key          : 'max-atomics',
            meaningNodes : [{ kind: 'text', text: 'Keep short.' }],
            type         : 'positive int | false'
          }
        ] }
      ]
    }
  ]
}))

const mountFacets = (options: { attachTo?: HTMLElement } = {}) =>
  mount(PerRuleFacets, {
    ...options,
    global : { components: { InlineProse }, stubs: { InlineRuleLink: true } }
  })

describe('PerRuleFacets', () => {
  afterEach(() => { window.location.hash = '' })

  it('renders one collapsible head per family, counting facets across its rules', () => {
    const heads = mountFacets().findAll('.per-rule-facets-head')
    expect(heads).toHaveLength(2)
    expect(heads[0].get('.per-rule-facets-label').text()).toContain('Generic')
    expect(heads[0].get('.per-rule-facets-count').text()).toBe('1 facet')
    expect(heads[1].get('.per-rule-facets-count').text()).toBe('3 facets')
  })

  it('starts collapsed and expands its section on click', async () => {
    const head = mountFacets().findAll('.per-rule-facets-head')[0]
    expect(head.attributes('aria-expanded')).toBe('false')
    await head.trigger('click')
    expect(head.attributes('aria-expanded')).toBe('true')
  })

  it('nests facets under a rule chip, keeping a generic scope as plain text', () => {
    const w = mountFacets()
    expect(w.get('.per-rule-facets-scope').text()).toBe('every rule')
    expect(w.findAllComponents(InlineRuleLink).map(c => c.props('slug')))
      .toStrictEqual(['reflow-calls', 'reflow-collections'])
    expect(w.findAll('.per-rule-facets-key').map(k => k.text()))
      .toStrictEqual(['enabled', 'max-args', 'keep-multiline-literals', 'max-atomics'])
  })

  it('renders each facet type, default, and rendered meaning', () => {
    const w       = mountFacets()
    const maxArgs = w.findAll('.per-rule-facets-entry')[1]
    expect(maxArgs.get('.per-rule-facets-type').text()).toBe('positive int | false')
    expect(maxArgs.get('.per-rule-facets-default-value').text()).toBe('3')
    expect(w.findAll('.per-rule-facets-entry')[2].get('.per-rule-facets-meaning').html())
      .toContain('<code>false</code>')
  })

  it('anchors every rule and facet, each carrying the permalink beside its name', () => {
    const w = mountFacets()
    expect(w.findAll('.per-rule-facets-rule-head').map(head => head.attributes('id')))
      .toStrictEqual(['every-rule', 'reflow-calls', 'reflow-collections'])
    expect(w.findAll('.per-rule-facets-term').map(term => term.attributes('id')))
      .toStrictEqual([
        'every-rule-enabled',
        'reflow-calls-max-args',
        'reflow-collections-keep-multiline-literals',
        'reflow-collections-max-atomics'
      ])
    expect(w.get('#reflow-calls-max-args a.header-anchor').attributes())
      .toMatchObject({ 'aria-label': 'Permalink to “max-args”', href: '#reflow-calls-max-args' })
  })

  it.each([['reflow-collections'], ['reflow-collections-max-atomics']])(
    'expands the family holding %s when the address bar names it',
    async fragment => {
      window.location.hash = `#${fragment}`
      const w = mountFacets()
      await nextTick()
      expect(w.findAll('.per-rule-facets-head').map(head => head.attributes('aria-expanded')))
        .toStrictEqual(['false', 'true'])
    }
  )

  it('scrolls to the facet it reveals, which the browser passed over while it sat hidden', async () => {
    const reached: string[] = []
    vi.spyOn(Element.prototype, 'scrollIntoView')
      .mockImplementation(function scroll(this: Element) { reached.push(this.id) })
    window.location.hash = '#reflow-collections-max-atomics'
    mountFacets({ attachTo: document.body })
    await nextTick()
    expect(reached).toStrictEqual(['reflow-collections-max-atomics'])
  })

  it('leaves every family collapsed for a fragment naming no anchor', async () => {
    window.location.hash = '#per-rule-facets'
    const w = mountFacets()
    await nextTick()
    expect(w.findAll('.per-rule-facets-head').map(head => head.attributes('aria-expanded')))
      .toStrictEqual(['false', 'false'])
  })

  it('carries the family accent through data-family', () => {
    expect(mountFacets().findAll('.per-rule-facets-section')[1].attributes('data-family')).toBe('layout')
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mountFacets().html())
  })
})
