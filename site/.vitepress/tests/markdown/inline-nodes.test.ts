import MarkdownIt from 'markdown-it'

import { glossaryPlugin }         from '../../lib/glossary/plugin'
import { blockNodes, inlineNodes } from '../../lib/markdown/inline-nodes'
import { proseMarkPlugin }        from '../../lib/markdown/prose-mark-plugin'
import { ruleLinkPlugin }         from '../../lib/rules/link-plugin'

const rules = new Map([
  ['align-equals', { family: 'alignment', href: '/rules/alignment/align-equals' }]
])

const hrefs      = new Map([['atomic', '/reference/glossary#atomic']])
const phrases    = new Map([['atom', 'atomic']])
const primitives = new Map([['source', { name: 'Source' }]])

function renderer(): MarkdownIt {
  const md = new MarkdownIt()
  md.use(ruleLinkPlugin(rules as never, primitives as never))
  md.use(glossaryPlugin(phrases, hrefs))
  md.use(proseMarkPlugin)
  return md
}

describe('inlineNodes', () => {
  it('walks a rule slug into a rule node whether it is backticked or wiki-linked', () => {
    expect(inlineNodes(renderer(), '`align-equals`'))
      .toEqual([{ kind: 'rule', slug: 'align-equals' }])
    expect(inlineNodes(renderer(), '[[align-equals]]'))
      .toEqual([{ kind: 'rule', slug: 'align-equals' }])
  })

  it('carries a primitive display name so the renderer needs no registry', () => {
    expect(inlineNodes(renderer(), '[[source]]'))
      .toEqual([{ kind: 'primitive', display: 'Source', slug: 'source' }])
  })

  it('walks a glossary term into a term node', () => {
    expect(inlineNodes(renderer(), 'an atom')).toEqual([
      { kind: 'text', text: 'an ' },
      { kind: 'term', slug: 'atomic', text: 'atom' }
    ])
  })

  it('nests a term inside its enclosing element rather than flattening it', () => {
    expect(inlineNodes(renderer(), '**an atom**')).toEqual([{
      kind     : 'el',
      attrs    : {},
      tag      : 'strong',
      children : [
        { kind: 'text', text: 'an ' },
        { kind: 'term', slug: 'atomic', text: 'atom' }
      ]
    }])
  })

  it('walks the Prose mark into a balanced span rather than raw html', () => {
    expect(inlineNodes(renderer(), 'Prose')).toEqual([{
      kind     : 'el',
      attrs    : { class: 'prose-mark' },
      tag      : 'span',
      children : [{ kind: 'text', text: 'Prose' }]
    }])
  })

  it('throws on a token it cannot map, so a new plugin fails the build', () => {
    const md = new MarkdownIt({ html: true })
    expect(() => inlineNodes(md, 'text with <b>raw html</b>')).toThrow(/cannot map/)
  })

  it('collapses a softbreak to a space, the shape every multi-line description carries', () => {
    expect(inlineNodes(renderer(), 'first\nsecond')).toEqual([
      { kind: 'text', text: 'first' },
      { kind: 'text', text: ' ' },
      { kind: 'text', text: 'second' }
    ])
  })

  it('walks block prose, flattening each paragraph inline run into the tree', () => {
    expect(blockNodes(renderer(), 'A lead over an atom.')).toEqual([{
      kind     : 'el',
      attrs    : {},
      tag      : 'p',
      children : [
        { kind: 'text', text: 'A lead over an ' },
        { kind: 'term', slug: 'atomic', text: 'atom' },
        { kind: 'text', text: '.' }
      ]
    }])
  })
})
