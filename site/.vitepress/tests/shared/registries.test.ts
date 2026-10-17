import MarkdownIt from 'markdown-it'

import * as head                from '../../lib/config/head'
import { sectionHeadingPlugin } from '../../lib/markdown/section-heading-plugin'
import { enumeratePages }       from '../../lib/og/pages'
import * as registries          from '../../lib/shared/registries'
import { fixtureDir }           from '../support'

import type { PageData } from 'vitepress'

describe('a relabeled section', () => {
  const guide = registries.SECTIONS.find(section => section.slug === 'guide') as { label: string }
  const label = guide.label

  beforeEach(() => { guide.label = 'Walkthrough' })
  afterEach(() => { guide.label = label })

  it('heads its index page with the new label', () => {
    expect(new MarkdownIt().use(sectionHeadingPlugin).render('Body.', { relativePath: 'guide/index.md' }))
      .toBe('<h1>Walkthrough</h1>\n<p>Body.</p>\n')
  })

  it('titles its index page with the new label', () => {
    const data = { frontmatter: {}, relativePath: 'guide/index.md', title: '' } as PageData
    head.describePage(data)
    expect(data.title).toBe('Walkthrough')
  })

  it('names its og breadcrumb and index card with the new label', () => {
    expect(enumeratePages(fixtureDir(`${import.meta.dirname}/../og`), ['guide/index.md']))
      .toMatchObject([{ breadcrumb: ['Walkthrough'], title: 'Walkthrough' }])
  })
})

describe('sectionOf', () => {
  it.each([
    ['index.md',                        null],
    ['prototypes/icons/index.md',       null],
    ['reference/cli.md',                'reference'],
    ['rules/alignment/align-equals.md', 'rules'],
    ['sandbox/index.md',                'sandbox']
  ] as const)('maps %s to %s', (rel, expected) => {
    expect(registries.sectionOf(rel)).toBe(expected)
  })
})

describe('GLOSSARY_FAMILY_META', () => {
  it.each([...registries.FAMILY_ORDER])('carries %s unchanged from registries.FAMILY_META', (family) => {
    expect(registries.GLOSSARY_FAMILY_META[family]).toMatchObject(registries.FAMILY_META[family])
  })
})

describe('registry types', () => {
  it('FAMILY_META keys equal the RuleFamily union', () => {
    expectTypeOf<keyof typeof registries.FAMILY_META>().toEqualTypeOf<registries.RuleFamily>()
  })

  it('GLOSSARY_FAMILY_META keys equal the GlossaryFamily union', () => {
    expectTypeOf<keyof typeof registries.GLOSSARY_FAMILY_META>().toEqualTypeOf<registries.GlossaryFamily>()
  })
})
