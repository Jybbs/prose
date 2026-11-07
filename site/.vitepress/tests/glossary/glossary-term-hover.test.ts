// @vitest-environment happy-dom
import { mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h }     from 'vue'

import { provideAriaHidden } from '../../lib/composables/use-aria-hidden'
import GlossaryTerm          from '../../theme/components/glossary/GlossaryTerm.vue'

vi.mock('vitepress', () => ({ defineLoader: () => {}, useRoute: () => ({ path: '/architecture/' }) }))

vi.mock('../../lib/glossary/glossary.data', () => ({
  data: {
    entries: {
      atomic: {
        aliases         : [],
        definitionHtml  : '<p>An indivisible literal.</p>',
        definitionNodes : [],
        families        : ['engine'],
        href            : '/reference/glossary#atomic',
        initial         : 'A',
        primaryFamily   : 'engine',
        slug            : 'atomic'
      }
    }
  }
}))

// Renders the popup's content in place, so the assertions read the content
// the term passes the popup and the theme it names.
const Popup = {
  props    : ['theme'],
  template : '<div :data-popup="theme"><slot name="trigger" :props="{}" /><slot /></div>'
}

describe('GlossaryTerm popup wiring', () => {
  let w: VueWrapper

  beforeEach(() => {
    w = mount(GlossaryTerm, {
      global : { stubs: { Popup } },
      props  : { slug: 'atomic' },
      slots  : { default: () => 'atom' }
    })
  })

  it('renders the anchor carrying the term text', () => {
    expect(w.get('.glossary-anchor').text()).toBe('atom')
  })

  it('hands the popup the definition and the glossary theme', () => {
    expect(w.attributes('data-popup')).toBe('glossary')
    expect(w.get('.glossary-tooltip-body').text()).toBe('An indivisible literal.')
    expect(w.find('a[href="/reference/glossary#atomic"]').exists()).toBe(true)
  })

  it('heads the popup with the title, each family nib, a middle dot, and the Glossary kicker', () => {
    const head = w.get('.glossary-tooltip-head')
    expect(head.get('.glossary-tooltip-title').text()).toBe('atomic')
    expect(head.findAll('.glossary-tooltip-nib').map(nib => [nib.attributes('data-family'), nib.get('svg').attributes('aria-label')]))
      .toStrictEqual([['engine', 'Engine']])
    expect(head.get('.middle-dot').text()).toBe('·')
    expect(head.get('.kicker').text()).toBe('Glossary')
  })

  it('draws Read More in the link style', () => {
    expect(w.get('a[href="/reference/glossary#atomic"] .nib-stroke').text()).toBe('Read more →')
  })
})

describe('GlossaryTerm tab order', () => {
  const anchor = (hidden: boolean) => {
    const Child = defineComponent({
      setup: () => () => h(GlossaryTerm, { slug: 'atomic' }, () => 'atom')
    })
    const Parent = defineComponent({
      setup() {
        provideAriaHidden(hidden)
        return () => h(Child)
      }
    })
    return mount(Parent).get('.glossary-anchor')
  }

  it('stays focusable in ordinary prose', () => {
    expect(anchor(false).attributes('tabindex')).toBe('0')
  })

  it('leaves the tab order inside an aria-hidden subtree', () => {
    expect(anchor(true).attributes('tabindex')).toBe('-1')
  })
})
