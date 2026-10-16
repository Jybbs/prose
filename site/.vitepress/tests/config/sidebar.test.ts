import fs   from 'node:fs'
import path from 'node:path'

import type { DefaultTheme } from 'vitepress'

import type { DiscoveredPrimitive } from '../../lib/architecture/discovery'
import { buildSidebar }             from '../../lib/config/sidebar'
import type { DiscoveredRule }      from '../../lib/rules/discovery'
import { isContentPage }            from '../../lib/shared/content-page'
import { siteDir }                  from '../../lib/shared/paths'
import { fixtureDir }               from '../support'

const rules: readonly DiscoveredRule[] = [
  {
    caption  : 'Align consecutive assignments',
    family   : 'alignment',
    href     : '/rules/alignment/align-equals',
    lints    : false,
    name     : 'Align Equals',
    related  : [],
    slug     : 'align-equals'
  },
  {
    caption  : 'Alphabetize sibling entries',
    family   : 'ordering',
    href     : '/rules/ordering/alphabetize-siblings',
    lints    : false,
    name     : 'Alphabetize Siblings',
    related  : [],
    slug     : 'alphabetize-siblings'
  }
]

const primitives: readonly Pick<DiscoveredPrimitive, 'slug' | 'stability' | 'title'>[] = [
  { slug: 'aligner',  stability: 'public',   title: 'The Aligner Module' },
  { slug: 'pipeline', stability: 'internal', title: 'The Pipeline Type'  }
]

const src     = siteDir(import.meta.url)
const sidebar = buildSidebar(primitives, rules, src) as Record<string, DefaultTheme.SidebarItem[]>

describe('buildSidebar', () => {
  it('builds the route-keyed sidebar tree', () => {
    expect(sidebar).toMatchSnapshot()
  })

  it.each(['guide', 'integrations', 'reference'])('covers every %s content page', section => {
    const links = sidebar[`/${section}/`].flatMap(group => group.items ?? []).map(item => item.link)
    const pages = fs.readdirSync(path.join(src, section)).filter(isContentPage)
    expect(pages.length).toBeGreaterThan(0)
    for (const page of pages) {
      expect(links).toContain(`/${section}/${path.basename(page, '.md')}`)
    }
  })

  it('throws on a flat-section page with no H1', () => {
    expect(() => buildSidebar(primitives, rules, fixtureDir(import.meta.dirname, 'h1-less')))
      .toThrow(/guide\/broken\.md has no H1/)
  })
})
