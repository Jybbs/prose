import path from 'node:path'

import { defineLoader } from 'vitepress'

import { markdownH1 }                   from '../markdown/h1'
import { inlineNodes, type InlineNode } from '../markdown/inline-nodes'
import { getRenderer }                  from '../markdown/renderer'
import { matterPages }                  from '../shared/content-page'
import { siteDir }                      from '../shared/paths'
import type { SectionSlug }             from '../shared/registries'
import { requireString }                from '../shared/require-string'
import { sectionRoute }                 from '../shared/routes'

const SECTIONS = ['guide', 'reference'] as const satisfies readonly SectionSlug[]

export type GlanceSection = (typeof SECTIONS)[number]

export interface GlanceEntry {
  descriptionNodes : InlineNode[]
  href             : string
  title            : string
}

type Glance = Record<GlanceSection, readonly GlanceEntry[]>

declare const data: Glance
export { data }

const site = siteDir(import.meta.url)

// The description follows the linked title mid-sentence, so its first
// letter drops the capital the page's `<meta description>` keeps.
const continuing = (description: string): string =>
  description.charAt(0).toLowerCase() + description.slice(1)

export default defineLoader({
  watch: SECTIONS.map(section => path.join(site, section, '*.md')),
  async load(): Promise<Glance> {
    const md = await getRenderer()
    const entries = (section: GlanceSection): readonly GlanceEntry[] =>
      matterPages(path.join(site, section))
        .map(({ content, data: fm, slug }) => ({
          descriptionNodes : inlineNodes(md, continuing(requireString(
            fm.description,
            `${section}/${slug}.md is missing the description frontmatter its section glance reads`
          ))),
          href             : `${sectionRoute(section)}${slug}`,
          title            : requireString(
            markdownH1(content),
            `${section}/${slug}.md has no H1 for its section glance`
          )
        }))
        .toSorted((a, b) => a.title.localeCompare(b.title))
    return { guide: entries('guide'), reference: entries('reference') }
  }
})
