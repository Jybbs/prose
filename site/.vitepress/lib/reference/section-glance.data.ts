import fs   from 'node:fs'
import path from 'node:path'

import matter           from 'gray-matter'
import { defineLoader } from 'vitepress'

import { markdownH1 }                          from '../markdown/h1'
import { inlineNodes, type InlineNode }        from '../markdown/inline-nodes'
import { bareInlineNodes, inlineText }         from '../markdown/inline-text'
import { getRenderer }                         from '../markdown/renderer'
import { contentPages, frontmatterDescription } from '../shared/content-page'
import { siteDir }                             from '../shared/paths'
import { SECTIONS, type SectionSlug }          from '../shared/registries'
import { requireString }                       from '../shared/require-string'
import { sectionRoute }                        from '../shared/routes'

export interface GlanceEntry {
  descriptionNodes : InlineNode[]
  href             : string
  titleNodes       : InlineNode[]
}

type Glance = Record<SectionSlug, readonly GlanceEntry[]>

declare const data: Glance
export { data }

const site = siteDir(import.meta.url)

// The description follows the linked title mid-sentence, so its first
// letter drops the capital the page's `<meta description>` keeps, unless it
// opens an acronym such as `SARIF`.
export const continuing = (description: string): string =>
  description.replace(/^[A-Z](?![A-Z])/, letter => letter.toLowerCase())

// Lists a section's own pages beside its index, then the index page of each
// folder inside it, such as a rule family under `rules/`, as a page file
// relative to the site root beside the route it serves.
function sectionPages(section: SectionSlug): { file: string, href: string }[] {
  const route   = sectionRoute(section)
  const entries = fs.readdirSync(path.join(site, section), { withFileTypes: true })
  const folders = entries
    .filter(entry => entry.isDirectory())
    .map(({ name }) => ({ file: path.join(section, name, 'index.md'), href: `${route}${name}/` }))
    .filter(({ file }) => fs.existsSync(path.join(site, file)))
  const pages   = contentPages(path.join(site, section)).map(name => ({
    file : path.join(section, name),
    href : `${route}${path.basename(name, '.md')}`
  }))
  return [...pages, ...folders]
}

export default defineLoader({
  watch: SECTIONS.flatMap(({ slug }) => [
    path.join(site, slug, '*.md'),
    path.join(site, slug, '*', 'index.md')
  ]),
  async load(): Promise<Glance> {
    const md = await getRenderer()
    const entries = (section: SectionSlug): readonly GlanceEntry[] =>
      sectionPages(section)
        .map(({ file, href }) => {
          const { content, data: fm } = matter.read(path.join(site, file))
          const description = requireString(
            frontmatterDescription(fm),
            `${file} is missing the description frontmatter its section glance reads`
          )
          const title = requireString(markdownH1(content), `${file} has no H1 for its section glance`)
          return { description, href, title }
        })
        .toSorted((a, b) => inlineText(a.title).localeCompare(inlineText(b.title)))
        .map(({ description, href, title }) => ({
          descriptionNodes : inlineNodes(md, continuing(description)),
          href,
          titleNodes       : bareInlineNodes(title)
        }))
    return Object.fromEntries(SECTIONS.map(({ slug }) => [slug, entries(slug)])) as Glance
  }
})
