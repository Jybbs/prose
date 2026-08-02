import fs   from 'node:fs'
import path from 'node:path'

import type { DefaultTheme } from 'vitepress'

import { buildSidebar }             from '../../lib/config/sidebar'
import type { DiscoveredPrimitive } from '../../lib/primitives/discovery'
import type { DiscoveredRule }      from '../../lib/rules/discovery'
import { isContentPage }            from '../../lib/shared/content-page'
import { siteDir }                  from '../../lib/shared/paths'
import { fixtureDir }               from '../support'

const rules: readonly DiscoveredRule[] = [
  {
    caption  : 'Align consecutive assignments',
    category : 'auto-fix',
    family   : 'alignment',
    href     : '/rules/alignment/align-equals',
    lints    : false,
    related  : [],
    slug     : 'align-equals'
  },
  {
    caption  : 'Alphabetize sibling entries',
    category : 'auto-fix',
    family   : 'ordering',
    href     : '/rules/ordering/alphabetize',
    lints    : false,
    related  : [],
    slug     : 'alphabetize'
  }
]

const primitives: readonly Pick<DiscoveredPrimitive, 'name' | 'slug' | 'stability'>[] = [
  { name: 'Aligner',  slug: 'aligner',  stability: 'public'   },
  { name: 'Pipeline', slug: 'pipeline', stability: 'internal' }
]

const src     = siteDir(import.meta.url)
const sidebar = buildSidebar(primitives, rules, src) as Record<string, DefaultTheme.SidebarItem[]>

describe('buildSidebar', () => {
  it('builds the route-keyed sidebar tree', () => {
    expect(sidebar).toMatchSnapshot()
  })

  it.each(['integrations', 'reference', 'usage'])('covers every %s content page', section => {
    const links = sidebar[`/${section}/`].flatMap(group => group.items ?? []).map(item => item.link)
    const pages = fs.readdirSync(path.join(src, section)).filter(isContentPage)
    expect(pages.length).toBeGreaterThan(0)
    for (const page of pages) {
      expect(links).toContain(`/${section}/${path.basename(page, '.md')}`)
    }
  })

  it('throws on a flat-section page with no H1', () => {
    expect(() => buildSidebar(primitives, rules, fixtureDir(import.meta.dirname, 'h1-less')))
      .toThrow(/usage\/broken\.md has no H1/)
  })
})
