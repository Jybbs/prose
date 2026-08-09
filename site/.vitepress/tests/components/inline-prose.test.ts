// @vitest-environment happy-dom
import { mount }              from '@vue/test-utils'
import { defineComponent, h } from 'vue'

import { provideAriaHidden } from '../../lib/composables/use-aria-hidden'
import GlossaryTerm          from '../../theme/components/glossary/GlossaryTerm.vue'
import InlineProse           from '../../theme/components/base/InlineProse.vue'
import InlineRuleLink        from '../../theme/components/rules/InlineRuleLink.vue'

import type { InlineNode } from '../../lib/markdown/inline-nodes'

vi.mock('../../lib/glossary/glossary.data', () => ({ data: { entries: [] } }))
vi.mock('../../lib/rules/rules.data', () => ({ data: {} }))

const STUBS = { global: { stubs: { GlossaryTerm: true, InlineRuleLink: true } } }

const render = (nodes: InlineNode[]) => mount(InlineProse, { ...STUBS, props: { nodes } })

const renderHidden = (nodes: InlineNode[]) => mount(
  defineComponent({
    setup: () => { provideAriaHidden(true); return () => h(InlineProse, { nodes }) }
  }),
  STUBS
)

describe('InlineProse', () => {
  it('renders every node kind, so no branch silently drops out of the v-if chain', () => {
    const html = render([
      { kind: 'text', text: 'A ' },
      { kind: 'code', text: 'x' },
      { kind: 'el', tag: 'strong', attrs: {}, children: [{ kind: 'text', text: 'bold' }] },
      { kind: 'primitive', display: 'Source', slug: 'source' }
    ]).html()

    expect(html).toContain('<code>x</code>')
    expect(html).toContain('<strong>bold</strong>')
    expect(html).toContain('href="/primitives/source"')
    expect(html).toContain('A')
  })

  it('mounts a term as a real component rather than inert markup', () => {
    const w = render([{ kind: 'term', slug: 'atomic', text: 'atom' }])
    expect(w.findComponent(GlossaryTerm).props('slug')).toBe('atomic')
  })

  it('mounts a rule chip as a real component', () => {
    const w = render([{ kind: 'rule', slug: 'align-equals' }])
    expect(w.findComponent(InlineRuleLink).props('slug')).toBe('align-equals')
  })

  it('recurses so a term nested inside an element still mounts', () => {
    const w = render([{
      kind     : 'el',
      tag      : 'strong',
      attrs    : {},
      children : [{ kind: 'term', slug: 'atomic', text: 'atom' }]
    }])
    expect(w.findComponent(GlossaryTerm).props('slug')).toBe('atomic')
  })

  it('drops a primitive link out of the tab order inside an aria-hidden subtree', () => {
    const nodes: InlineNode[] = [{ kind: 'primitive', display: 'Source', slug: 'source' }]
    expect(renderHidden(nodes).get('a').attributes('tabindex')).toBe('-1')
    expect(render(nodes).get('a').attributes('tabindex')).toBeUndefined()
  })

  it('drops an element anchor out of the tab order inside an aria-hidden subtree', () => {
    const nodes: InlineNode[] = [{
      kind     : 'el',
      tag      : 'a',
      attrs    : { href: '/usage/' },
      children : [{ kind: 'text', text: 'Usage' }]
    }]
    expect(renderHidden(nodes).get('a').attributes('tabindex')).toBe('-1')
  })

  it('leaves an element\'s own tabindex alone outside an aria-hidden subtree', () => {
    const w = render([{
      kind     : 'el',
      tag      : 'a',
      attrs    : { href: '/usage/', tabindex: '0' },
      children : [{ kind: 'text', text: 'Usage' }]
    }])
    expect(w.get('a').attributes('tabindex')).toBe('0')
  })

  it('carries an element attribute through to the rendered tag', () => {
    const w = render([{
      kind     : 'el',
      tag      : 'span',
      attrs    : { class: 'prose-mark' },
      children : [{ kind: 'text', text: 'Prose' }]
    }])
    expect(w.html()).toContain('<span class="prose-mark">Prose</span>')
  })
})
