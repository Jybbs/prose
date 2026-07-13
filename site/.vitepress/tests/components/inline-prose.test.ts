// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import GlossaryTerm   from '../../theme/components/glossary/GlossaryTerm.vue'
import InlineProse    from '../../theme/components/base/InlineProse.vue'
import InlineRuleLink from '../../theme/components/rules/InlineRuleLink.vue'

import type { InlineNode } from '../../lib/markdown/inline-nodes'

vi.mock('../../lib/glossary/glossary.data', () => ({ data: { entries: [] } }))
vi.mock('../../lib/rules/rules.data', () => ({ data: {} }))

const render = (nodes: InlineNode[]) =>
  mount(InlineProse, {
    global : { stubs: { GlossaryTerm: true, InlineRuleLink: true } },
    props  : { nodes }
  })

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
