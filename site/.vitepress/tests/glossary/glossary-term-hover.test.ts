// @vitest-environment happy-dom
import { mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h }     from 'vue'

import { provideAriaHidden } from '../../lib/composables/use-aria-hidden'
import GlossaryTerm          from '../../theme/components/glossary/GlossaryTerm.vue'

vi.mock('vitepress', () => ({ useRoute: () => ({ path: '/architecture/' }) }))

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

// FloatingVue's popper cannot open inside happy-dom, so the assertions stop
// at the directive receiving the binding and the binding carrying the
// definition, the same wiring the compiled pages hover through.
describe('GlossaryTerm tooltip wiring', () => {
  interface TooltipBinding {
    value: { content: string, theme: string }
  }

  const tooltip = vi.fn<(el: Element, binding: TooltipBinding) => void>()

  let w: VueWrapper

  beforeEach(() => {
    w = mount(GlossaryTerm, {
      global : { directives: { tooltip } },
      props  : { slug: 'atomic' },
      slots  : { default: () => 'atom' }
    })
  })

  it('renders the anchor carrying the term text', () => {
    expect(w.get('.glossary-anchor').text()).toBe('atom')
  })

  it('hands the tooltip directive the definition and the glossary theme', () => {
    const binding = tooltip.mock.calls[0][1].value
    expect(binding.theme).toBe('glossary')
    expect(binding.content).toContain('An indivisible literal.')
    expect(binding.content).toContain('href="/reference/glossary#atomic"')
  })

  it('heads the tooltip with the title, each family nib, a middle dot, and the Glossary kicker', () => {
    const head = new DOMParser().parseFromString(tooltip.mock.calls[0][1].value.content, 'text/html')
      .querySelector('.glossary-tooltip-head')!
    expect(head.querySelector('.glossary-tooltip-title')?.textContent).toBe('atomic')
    expect([...head.querySelectorAll<HTMLElement>('.glossary-tooltip-nib')].map(nib => [nib.dataset.family, nib.querySelector('svg')?.getAttribute('aria-label')]))
      .toStrictEqual([['engine', 'Engine']])
    expect(head.querySelector('.middle-dot')?.textContent).toBe('·')
    expect(head.querySelector('.kicker')?.textContent).toBe('Glossary')
  })

  it('draws Read More in the link style', () => {
    expect(tooltip.mock.calls[0][1].value.content)
      .toContain('<a href="/reference/glossary#atomic" class="glossary-tooltip-link"><span class="nib-stroke">Read more →</span></a>')
  })
})

describe('GlossaryTerm tab order', () => {
  const tooltip = vi.fn<() => void>()

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
    return mount(Parent, { global: { directives: { tooltip } } }).get('.glossary-anchor')
  }

  it('stays focusable in ordinary prose', () => {
    expect(anchor(false).attributes('tabindex')).toBe('0')
  })

  it('leaves the tab order inside an aria-hidden subtree', () => {
    expect(anchor(true).attributes('tabindex')).toBe('-1')
  })
})
