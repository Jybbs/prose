import MarkdownIt                from 'markdown-it'
import type { MarkdownRenderer } from 'vitepress'

import { glossaryPlugin }      from '../../lib/glossary/plugin'
import { bodyLinkPlugin }      from '../../lib/markdown/body-link-plugin'
import { proseMarkPlugin }     from '../../lib/markdown/prose-mark-plugin'
import { renderInlineHtml }    from '../../lib/markdown/renderer'
import type { DiscoveredRule } from '../../lib/rules/discovery'
import { ruleLinkPlugin }      from '../../lib/rules/link-plugin'

const render = (configure: (md: MarkdownIt) => void, src: string, env: object = {}): string => {
  const md = new MarkdownIt()
  configure(md)
  return md.render(src, env)
}

describe('proseMarkPlugin', () => {
  it('wraps a standalone Prose in a prose-mark span', () => {
    expect(render(md => md.use(proseMarkPlugin), 'Prose formats code'))
      .toContain('<span class="prose-mark">Prose</span>')
  })

  it('keeps hyphenated and snake_case compounds literal', () => {
    expect(render(md => md.use(proseMarkPlugin), 'the prose-mark class and prose_mark name'))
      .not.toContain('<span class="prose-mark">')
  })
})

describe('bodyLinkPlugin', () => {
  it('adds the body-link class to inline links', () => {
    expect(render(md => md.use(bodyLinkPlugin), '[docs](/x)')).toContain('class="body-link underline-draw"')
  })
})

describe('glossaryPlugin', () => {
  const map    = new Map([['atom', 'atomic']])
  const plugin = glossaryPlugin(map, new Map([['atomic', '/reference/glossary#atomic']]))

  it('decorates the first occurrence of a glossary phrase', () => {
    expect(render(md => md.use(plugin), 'an atom here'))
      .toContain('<GlossaryTerm slug="atomic">atom</GlossaryTerm>')
  })

  it('decorates a phrase only once per page', () => {
    const html = render(md => md.use(plugin), 'atom and atom')
    expect(html.match(/<GlossaryTerm/g)).toHaveLength(1)
  })

  it('keeps a phrase inside a compound literal', () => {
    expect(render(md => md.use(plugin), 'an atom-splitter here')).not.toContain('GlossaryTerm')
  })

  it('emits an inert glossary anchor under the inertHtml env', () => {
    expect(render(md => md.use(plugin), 'an atom here', { inertHtml: true }))
      .toContain('<a class="glossary-term" data-term="atomic" href="/reference/glossary#atomic">atom</a>')
  })

  it('emits an inert glossary span when the entry has no href', () => {
    const bare = glossaryPlugin(map, new Map())
    expect(render(md => md.use(bare), 'an atom here', { inertHtml: true }))
      .toContain('<span class="glossary-term" data-term="atomic">atom</span>')
  })

  it('renders inert through the loader wrapper', () => {
    const md = new MarkdownIt()
    md.use(plugin)
    expect(renderInlineHtml(md as unknown as MarkdownRenderer, 'an atom here'))
      .toContain('<a class="glossary-term"')
  })

  it('throws on an empty phrase map', () => {
    expect(() => glossaryPlugin(new Map(), new Map())).toThrow(/empty phrase map/)
  })
})

describe('ruleLinkPlugin', () => {
  const rules  = new Map<string, Pick<DiscoveredRule, 'family' | 'href'>>([
    ['align-equals', { family: 'alignment', href: '/rules/alignment/align-equals' }]
  ])
  const plugin = ruleLinkPlugin(rules, new Map([['aligner', { name: 'Aligner' }]]))
  const run    = (src: string, env: object = {}): string => render(md => md.use(plugin), src, env)

  it('renders a rule wiki-link as an InlineRuleLink', () => {
    expect(run('see [[align-equals]]')).toContain('<InlineRuleLink slug="align-equals" />')
  })

  it('renders a primitive wiki-link as a body link', () => {
    expect(run('see [[aligner]]'))
      .toContain('<a class="body-link underline-draw" href="/primitives/aligner">')
  })

  it('promotes an inline-code rule slug to a doc link', () => {
    expect(run('the `align-equals` rule')).toContain('<InlineRuleLink slug="align-equals" />')
  })

  it('emits an inert rule anchor under the inertHtml env', () => {
    expect(run('see [[align-equals]]', { inertHtml: true })).toContain(
      '<a class="rule-link" data-family="alignment" href="/rules/alignment/align-equals">align-equals</a>'
    )
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

  it('throws on a capitalized unknown wiki-link slug', () => {
    expect(() => run('see [[Ghost]]')).toThrow(/Unknown slug/)
  })
})
