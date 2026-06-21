import MarkdownIt from 'markdown-it'

import { glossaryPlugin }  from '../../lib/glossary/plugin'
import { bodyLinkPlugin }  from '../../lib/markdown/body-link-plugin'
import { proseMarkPlugin } from '../../lib/markdown/prose-mark-plugin'
import { ruleLinkPlugin }  from '../../lib/rules/link-plugin'

const render = (configure: (md: MarkdownIt) => void, src: string): string => {
  const md = new MarkdownIt()
  configure(md)
  return md.render(src)
}

describe('proseMarkPlugin', () => {
  it('wraps a standalone Prose in a prose-mark span', () => {
    expect(render(md => md.use(proseMarkPlugin), 'Prose formats code'))
      .toContain('<span class="prose-mark">Prose</span>')
  })
})

describe('bodyLinkPlugin', () => {
  it('adds the body-link class to inline links', () => {
    expect(render(md => md.use(bodyLinkPlugin), '[docs](/x)')).toContain('class="body-link"')
  })
})

describe('glossaryPlugin', () => {
  const map = new Map([['atom', 'atomic']])

  it('decorates the first occurrence of a glossary phrase', () => {
    expect(render(md => md.use(glossaryPlugin(map)), 'an atom here'))
      .toContain('<GlossaryTerm slug="atomic">atom</GlossaryTerm>')
  })

  it('decorates a phrase only once per page', () => {
    const html = render(md => md.use(glossaryPlugin(map)), 'atom and atom')
    expect(html.match(/<GlossaryTerm/g)).toHaveLength(1)
  })

  it('throws on an empty phrase map', () => {
    expect(() => glossaryPlugin(new Map())).toThrow(/empty phrase map/)
  })
})

describe('ruleLinkPlugin', () => {
  const plugin = ruleLinkPlugin(new Set(['align-equals']), new Map([['aligner', 'Aligner']]))
  const run    = (src: string): string => render(md => md.use(plugin), src)

  it('renders a rule wiki-link as an InlineRuleLink', () => {
    expect(run('see [[align-equals]]')).toContain('<InlineRuleLink slug="align-equals" />')
  })

  it('renders a primitive wiki-link as a body link', () => {
    expect(run('see [[aligner]]')).toContain('href="/primitives/aligner"')
  })

  it('promotes an inline-code rule slug to a doc link', () => {
    expect(run('the `align-equals` rule')).toContain('<InlineRuleLink slug="align-equals" />')
  })

  it('leaves an unclosed wiki-link as literal text', () => {
    expect(run('see [[align-equals')).toContain('[[align-equals')
  })

  it('leaves a non-slug wiki-link as literal text', () => {
    expect(run('see [[Bad Slug]]')).toContain('[[Bad Slug]]')
  })

  it('throws on an unknown wiki-link slug', () => {
    expect(() => run('see [[ghost]]')).toThrow(/Unknown slug/)
  })
})
