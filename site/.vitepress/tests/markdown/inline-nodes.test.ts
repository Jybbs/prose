import markdownit, { type MarkdownIt } from 'markdown-it'

import { glossaryPlugin }          from '../../lib/glossary/plugin'
import { codeUnitPlugin }          from '../../lib/markdown/code-operators'
import { blockNodes, inlineNodes } from '../../lib/markdown/inline-nodes'
import { ruleLinkPlugin }          from '../../lib/rules/link-plugin'

const rules = new Map([
  ['align-equals', { family: 'alignment', href: '/rules/alignment/align-equals' }]
])

const hrefs      = new Map([['atomic', '/reference/glossary#atomic']])
const phrases    = new Map([['atom', 'atomic']])
const primitives = new Map([['source', { name: 'Source' }]])

function renderer(): MarkdownIt {
  const md = markdownit()
  md.use(ruleLinkPlugin(rules as never, primitives as never))
  md.use(glossaryPlugin(phrases, hrefs))
  return md
}

describe('inlineNodes', () => {
  it('walks a rule slug into a rule node whether it is backticked or wiki-linked', () => {
    expect(inlineNodes(renderer(), '`align-equals`'))
      .toStrictEqual([{ kind: 'rule', slug: 'align-equals' }])
    expect(inlineNodes(renderer(), '[[align-equals]]'))
      .toStrictEqual([{ kind: 'rule', slug: 'align-equals' }])
  })

  it('carries a primitive display name so the renderer needs no registry', () => {
    expect(inlineNodes(renderer(), '[[source]]'))
      .toStrictEqual([{ kind: 'primitive', display: 'Source', slug: 'source' }])
  })

  it('walks a glossary term into a term node', () => {
    expect(inlineNodes(renderer(), 'an atom')).toStrictEqual([
      { kind: 'text', text: 'an ' },
      { kind: 'term', slug: 'atomic', text: 'atom' }
    ])
  })

  it('nests a term inside its enclosing element rather than flattening it', () => {
    expect(inlineNodes(renderer(), '**an atom**')).toStrictEqual([{
      kind     : 'el',
      attrs    : {},
      tag      : 'strong',
      children : [
        { kind: 'text', text: 'an ' },
        { kind: 'term', slug: 'atomic', text: 'atom' }
      ]
    }])
  })

  it('throws on a token it cannot map, so a new plugin fails the build', () => {
    const md = markdownit({ html: true })
    expect(() => inlineNodes(md, 'text with <b>raw html</b>')).toThrow(/cannot map/)
  })

  it('collapses a softbreak to a space, the shape every multi-line description carries', () => {
    expect(inlineNodes(renderer(), 'first\nsecond')).toStrictEqual([
      { kind: 'text', text: 'first' },
      { kind: 'text', text: ' ' },
      { kind: 'text', text: 'second' }
    ])
  })

  it('holds every attribute as a string, an ordered list start included', () => {
    expect(inlineNodes(markdownit(), '[a](/b)')).toStrictEqual([{
      kind     : 'el',
      attrs    : { href: '/b' },
      tag      : 'a',
      children : [{ kind: 'text', text: 'a' }]
    }])
    expect(blockNodes(markdownit(), '3. x')[0]).toMatchObject({ attrs: { start: '3' }, tag: 'ol' })
  })

  it('walks block prose, flattening each paragraph inline run into the tree', () => {
    expect(blockNodes(renderer(), 'A lead over an atom.')).toStrictEqual([{
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

  it('holds a code span and the punctuation after it in one `code-unit` element', () => {
    expect(inlineNodes(markdownit().use(codeUnitPlugin), '`x`, then')).toStrictEqual([{
      attrs    : { class: 'code-unit' },
      children : [{ kind: 'code', text: 'x' }, { kind: 'text', text: ',' }],
      kind     : 'el',
      tag      : 'span'
    }, { kind: 'text', text: ' then' }])
  })
})
