// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

import { provideAriaHidden } from '../../lib/composables/use-aria-hidden'
import GlossaryTerm          from '../../theme/components/glossary/GlossaryTerm.vue'

vi.mock('vitepress', () => ({ useRoute: () => ({ path: '/primitives/' }) }))

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

  const w = mount(GlossaryTerm, {
    global : { directives: { tooltip } },
    props  : { slug: 'atomic' },
    slots  : { default: () => 'atom' }
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
