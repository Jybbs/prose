import MarkdownIt                                        from 'markdown-it'
import { createMarkdownRenderer, type MarkdownRenderer } from 'vitepress'

import { glossaryPlugin }          from '../../lib/glossary/plugin'
import { bodyLinkPlugin }          from '../../lib/markdown/body-link-plugin'
import { calloutPlugin }           from '../../lib/markdown/callout-plugin'
import { emblemPlugin }            from '../../lib/markdown/emblem-plugin'
import { inertEnv, plainTermsEnv } from '../../lib/markdown/inert-env'
import { renderPlainInlineHtml }   from '../../lib/markdown/renderer'
import { sectionHeadingPlugin }    from '../../lib/markdown/section-heading-plugin'
import type { DiscoveredRule }     from '../../lib/rules/discovery'
import { ruleLinkPlugin }          from '../../lib/rules/link-plugin'
import { emblemsOf }               from '../../lib/shared/emblems'
import { siteDir }                 from '../../lib/shared/paths'
import { SECTIONS }                from '../../lib/shared/registries'

const render = (configure: (md: MarkdownIt) => void, src: string, env: object = {}): string => {
  const md = new MarkdownIt()
  configure(md)
  return md.render(src, env)
}

describe('calloutPlugin', () => {
  const callout = async (src: string): Promise<string> => {
    const md = await createMarkdownRenderer(siteDir(import.meta.url), {
      config: renderer => { renderer.use(calloutPlugin) }
    })
    return md.render(src)
  }

  it.each([
    ['danger',  '†', 'Danger'],
    ['info',    '¶', 'Info'],
    ['tip',     '§', 'Tip'],
    ['warning', '†', 'Warning']
  ])('opens a %s callout on its mark, its kind word, and its title', async (kind, mark, label) => {
    await expect(callout(`::: ${kind} Holds \`x\`\nBody.\n:::`)).resolves.toContain(
      `<div class="${kind} custom-block"><p class="custom-block-title">`
      + `<span class="callout-mark" aria-hidden="true">${mark}</span>`
      + `<span class="callout-kind">${label}</span>`
      + '<span class="callout-title">Holds <code>x</code></span></p>')
  })

  it('leaves the title out of a callout the page gives none', async () => {
    await expect(callout('::: tip\nBody.\n:::'))
      .resolves.toContain('<span class="callout-kind">Tip</span></p>')
  })
})

describe('sectionHeadingPlugin', () => {
  const heading = (src: string, env: object): string =>
    render(md => md.use(sectionHeadingPlugin), src, env)

  it.each(SECTIONS)('heads $slug/index.md with $label', ({ label, slug }) => {
    expect(heading('Body.', { relativePath: `${slug}/index.md` }))
      .toBe(`<h1>${label}</h1>\n<p>Body.</p>\n`)
  })

  it.each([
    ['a page beneath a section index', 'guide/quick-start.md'],
    ['a page outside every section',   'prototypes/icons/index.md']
  ])('leaves %s alone', (_where, relativePath) => {
    expect(heading('Body.', { relativePath })).toBe('<p>Body.</p>\n')
  })

  it('leaves an index page whose frontmatter sets heading to false alone', () => {
    expect(heading('Body.', { frontmatter: { heading: false }, relativePath: 'sandbox/index.md' }))
      .toBe('<p>Body.</p>\n')
  })

  it('throws on an index page that types its own H1', () => {
    expect(() => heading('# Guide', { relativePath: 'guide/index.md' }))
      .toThrow('guide/index.md types its own H1, which SECTIONS supplies')
  })
})

describe('bodyLinkPlugin', () => {
  it('adds the body-link class to inline links', () => {
    expect(render(md => md.use(bodyLinkPlugin), '[docs](/x)')).toContain('class="body-link"')
  })
})

describe('emblemPlugin', () => {
  const plugin = emblemPlugin(emblemsOf([{ emoji: '🐧', name: 'Linux' }]))

  it('sets an emoji in body text as its labeled badge in the running-text chip', () => {
    expect(render(md => md.use(plugin), 'the entries with the 🧶 badge')).toContain(
      'the entries with the <span class="nib-chip" data-family="lint">'
      + '<svg class="nib-badge" width="16" height="16" viewBox="0 0 24 24" role="img" aria-label="Yarn">'
      + '<use href="#nib-yarn-small"></use></svg></span> badge'
    )
  })

  it('gives a platform badge no family, so it takes Ube', () => {
    expect(render(md => md.use(plugin), 'built for 🐧'))
      .toContain('<span class="nib-chip"><svg class="nib-badge"')
  })

  it.each([
    ['a heading',   '## The 🧶 Badge'],
    ['a code span', 'the `🧶` badge']
  ])('leaves an emoji inside %s as written', (_where, src) => {
    expect(render(md => md.use(plugin), src)).toContain('🧶')
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
      .toContain('<a class="glossary-anchor underline-draw" data-term="atomic" '
               + 'href="/reference/glossary#atomic">atom</a>')
  })

  it('emits an inert glossary span when the entry has no href', () => {
    const bare = glossaryPlugin(map, new Map())
    expect(render(md => md.use(bare), 'an atom here', { inertHtml: true }))
      .toContain('<span class="glossary-anchor underline-draw" data-term="atomic">atom</span>')
  })

  it('flattens a phrase to plain text under the plain-terms env', () => {
    const html = render(md => md.use(plugin), 'an atom here', plainTermsEnv())
    expect(html).not.toContain('glossary-anchor')
    expect(html).toContain('an atom here')
  })

  it('keeps the glossary anchor under the inert env', () => {
    expect(render(md => md.use(plugin), 'an atom here', inertEnv()))
      .toContain('<a class="glossary-anchor underline-draw"')
  })

  it('renders flattened terms through the plain loader wrapper', () => {
    const md = new MarkdownIt()
    md.use(plugin)
    expect(renderPlainInlineHtml(md as unknown as MarkdownRenderer, 'an atom here'))
      .not.toContain('glossary-anchor')
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
      .toContain('<a class="body-link" href="/architecture/aligner">')
  })

  it('promotes an inline-code rule slug to a doc link', () => {
    expect(run('the `align-equals` rule')).toContain('<InlineRuleLink slug="align-equals" />')
  })

  it('emits an inert rule chip with its family badge under the inertHtml env', () => {
    expect(run('see [[align-equals]]', { inertHtml: true })).toContain(
      '<a class="rule-link" data-family="alignment" href="/rules/alignment/align-equals">'
      + '<svg class="nib-badge" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">'
      + '<use href="#nib-ladder-small"></use></svg>align-equals</a>'
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
