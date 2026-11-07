import markdownit, { type Env, type MarkdownIt }         from 'markdown-it'
import { createMarkdownRenderer, type MarkdownRenderer } from 'vitepress'

import { glossaryPlugin }                from '../../lib/glossary/plugin'
import { bodyLinkPlugin }                from '../../lib/markdown/body-link-plugin'
import { calloutPlugin }                 from '../../lib/markdown/callout-plugin'
import { codeOperatorPlugin, codeParts } from '../../lib/markdown/code-operators'
import { emblemPlugin }                  from '../../lib/markdown/emblem-plugin'
import { inertEnv, plainTermsEnv }       from '../../lib/markdown/inert-env'
import { proseMarkPlugin }               from '../../lib/markdown/prose-mark-plugin'
import { renderPlainInlineHtml }         from '../../lib/markdown/renderer'
import { scrollStopPlugin }              from '../../lib/markdown/scroll-stop-plugin'
import { sectionHeadingPlugin }          from '../../lib/markdown/section-heading-plugin'
import type { DiscoveredRule }           from '../../lib/rules/discovery'
import { ruleLinkPlugin }                from '../../lib/rules/link-plugin'
import { emblemsOf }                     from '../../lib/shared/emblems'
import { siteDir }                       from '../../lib/shared/paths'
import { SECTIONS }                      from '../../lib/shared/registries'

const render = (configure: (md: MarkdownIt) => void, src: string, env: Env = {}): string => {
  const md = markdownit()
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
      + '<span class="middle-dot" aria-hidden="true">·</span>'
      + '<span class="callout-title">Holds <code>x</code></span></p>')
  })

  it('resolves a reference link in a callout title', async () => {
    await expect(callout('::: tip See [x][y]\nBody.\n:::\n\n[y]: /z'))
      .resolves.toContain('<span class="callout-title">See <a href="/z.html">x</a></span>')
  })

  it('leaves the title out of a callout the page gives none', async () => {
    await expect(callout('::: tip\nBody.\n:::'))
      .resolves.toContain('<span class="callout-kind">Tip</span></p>')
  })
})

describe('codeParts', () => {
  it('splits a span into its operators and the runs between them', () => {
    expect(codeParts('x == y')).toStrictEqual([
      { op: false, text: 'x ' },
      { op: true,  text: '==' },
      { op: false, text: ' y' }
    ])
  })

  it('returns one plain run for an empty span', () => {
    expect(codeParts('')).toStrictEqual([{ op: false, text: '' }])
  })
})

describe('codeOperatorPlugin', () => {
  const code = (src: string): string => render(md => md.use(codeOperatorPlugin), src)

  it('holds a code span and the punctuation after it in one span', () => {
    expect(code('`x`, then')).toContain('<span class="code-unit"><code>x</code>,</span> then')
  })

  it('holds a code span in one span when separate copies of the module apply the plugin', async () => {
    vi.resetModules()
    const copy = await import('../../lib/markdown/code-operators')
    expect(render(md => md.use(codeOperatorPlugin).use(copy.codeOperatorPlugin), '`x`, then'))
      .toContain('<p><span class="code-unit"><code>x</code>,</span> then')
  })

  it('holds an emphasized code span and the punctuation after it in one span', () => {
    expect(code('(*`x`*), then')).toContain('(<span class="code-unit"><em><code>x</code></em>),</span> then')
  })

  it('closes an emphasis opened before the code span ahead of the span', () => {
    expect(code('*a `x`*)')).toContain('<em>a </em><span class="code-unit"><em><code>x</code></em>)</span>')
  })

  it('holds a link closing on its code span and the punctuation after it in one span', () => {
    expect(code('[`x`](/y), then')).toContain('<span class="code-unit"><a href="/y"><code>x</code></a>,</span> then')
  })

  it('leaves a code span with no punctuation after it unwrapped', () => {
    expect(code('`x` then')).not.toContain('code-unit')
  })

  it('leaves a link\'s code span as its direct child', () => {
    expect(code('[`x`, y](/z)')).toContain('<code>x</code>, y')
  })

  it.each([
    ['`x = 1`',                                  '<code>x <span class="code-op">=</span> 1</code>'],
    ['`=`',                                      '<code><span class="code-op">=</span></code>'],
    ['`a == b`',                                 '<code>a <span class="code-op">==</span> b</code>'],
    ['`x += 1`',                                 '<code>x <span class="code-op">+=</span> 1</code>'],
    ['`# note`',                                 '<code><span class="code-op">#</span> note</code>'],
    ['`x : int`',                                '<code>x <span class="code-op">:</span> int</code>'],
    ['`is not`',                                 '<code><span class="code-op">is not</span></code>'],
    ['`a not in b`',                             '<code>a <span class="code-op">not in</span> b</code>'],
    ['`x is not None`',                          '<code>x <span class="code-op">is not</span> None</code>'],
    ['`a in b`',                                 '<code>a <span class="code-op">in</span> b</code>'],
    ['`"a" in b`',                               '<code>&quot;a&quot; <span class="code-op">in</span> b</code>'],
    ['`\'a\' is b`',                             '<code>\'a\' <span class="code-op">is</span> b</code>'],
    ['`"#" in b`',                               '<code>&quot;#&quot; <span class="code-op">in</span> b</code>'],
    ['`"it\'s" in b`',                           '<code>&quot;it\'s&quot; <span class="code-op">in</span> b</code>'],
    ['`"a\\"b" in c`',                           '<code>&quot;a\\&quot;b&quot; <span class="code-op">in</span> c</code>'],
    ['`f"{a}" not in b`',                        '<code>f&quot;{a}&quot; <span class="code-op">not in</span> b</code>'],
    ['`a in b for b in c`',                      '<code>a <span class="code-op">in</span> b for b in c</code>'],
    ['`a in b # c in d`',                        '<code>a <span class="code-op">in</span> b <span class="code-op">#</span> c in d</code>'],
    ['`# per-attempt ceiling, in milliseconds`', '<code><span class="code-op">#</span> per-attempt ceiling, in milliseconds</code>'],
    ['`# a = b`',                                '<code><span class="code-op">#</span> a <span class="code-op">=</span> b</code>'],
    ['`"a = b"`',                                '<code>&quot;a <span class="code-op">=</span> b&quot;</code>'],
    ['`f"{ a = } in {b}"`',                      '<code>f&quot;{ a <span class="code-op">=</span> } in {b}&quot;</code>'],
    ['`a == b in c.`',                           '<code>a <span class="code-op">==</span> b in c.</code>']
  ])('sets each operator in %s in a code-op span', (src, html) => {
    expect(code(src)).toContain(html)
  })

  it.each([
    '`name=value`', '`a[1:2]`', '`https://prose.fyi`', '`<rule>`', '`y := 1`',
    '`[row for row in rows]`', '`5 diagnostics in 2 files.`', '`x is not None.`',
    '`"""Return whether the bag is empty."""`', '`f"{who} is {years}"`',
    '`f"{record!r} in {column:>8}"`', '`" and is in element content."`', '`\'a in b\'`',
    '`"a\\" in b"`'
  ])('leaves %s plain, since no alignment rule pads an operator between spaces in it', src => {
    expect(code(src)).not.toContain('code-op')
  })

  it('leaves a code span inside a link plain', () => {
    expect(code('[`x = 1`](/a) and `y = 2`'))
      .toContain('<a href="/a"><code>x = 1</code></a> and <code>y <span class="code-op">=</span> 2</code>')
  })

  it('escapes the code it splits', () => {
    expect(code('`a < b & c`')).toContain('<code>a <span class="code-op">&lt;</span> b &amp; c</code>')
  })
})

describe('proseMarkPlugin', () => {
  it('wraps a standalone Prose in a prose-mark span', () => {
    expect(render(md => md.use(proseMarkPlugin), 'Prose formats code'))
      .toContain('<span class="prose-mark">Prose</span>')
  })

  it.each(['the prose a reader skims', 'the Prose-mark class and Prose_mark name'])(
    'leaves the common noun and every compound in %s plain',
    src => { expect(render(md => md.use(proseMarkPlugin), src)).not.toContain('prose-mark') }
  )
})

describe('scrollStopPlugin', () => {
  it('gives a table a tab stop and leaves a paragraph alone', () => {
    const html = render(md => md.use(scrollStopPlugin), 'Text.\n\n| a |\n|---|\n| b |')
    expect(html).toContain('<table tabindex="0">')
    expect(html).toContain('<p>Text.</p>')
  })
})

describe('sectionHeadingPlugin', () => {
  const heading = (src: string, env: Env): string =>
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
      'the entries with the <span class="nib-chip ground" data-family="lint">'
      + '<svg class="nib-badge" width="16" height="16" viewBox="0 0 24 24" role="img" aria-label="Yarn">'
      + '<use href="#nib-yarn-small"></use></svg></span> badge'
    )
  })

  it('gives a platform badge no family, so it takes Ube', () => {
    expect(render(md => md.use(plugin), 'built for 🐧'))
      .toContain('<span class="nib-chip ground"><svg class="nib-badge"')
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
    const md = markdownit()
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
  const run    = (src: string, env: Env = {}): string => render(md => md.use(plugin), src, env)

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
      '<a class="rule-link ground" data-family="alignment" href="/rules/alignment/align-equals">'
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
