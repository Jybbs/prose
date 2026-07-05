import path from 'node:path'

import matter           from 'gray-matter'
import { defineLoader } from 'vitepress'

import { markdownH1 }                    from '../lib/markdown/h1'
import { getRenderer, renderInlineHtml } from '../lib/markdown/renderer'
import { contentPages }                  from '../lib/shared/content-page'
import { siteDir }                       from '../lib/shared/paths'
import { requireString }                 from '../lib/shared/require-string'

interface IntegrationCard {
  href        : string
  summaryHtml : string
  tagline     : string
  title       : string
}

declare const data: readonly IntegrationCard[]
export { data }

const directory = path.join(siteDir(import.meta.url), 'integrations')

const fieldMessage = (slug: string, key: string): string =>
  `integrations/${slug}.md is missing the ${key} frontmatter its index card reads`

export default defineLoader({
  watch: [`${directory}/*.md`],
  async load(): Promise<readonly IntegrationCard[]> {
    const md = await getRenderer()
    return contentPages(directory).map(file => {
      const page    = matter.read(path.join(directory, file))
      const slug    = path.basename(file, '.md')
      const summary = requireString(page.data.summary, fieldMessage(slug, 'summary'))
      const title   = requireString(
        markdownH1(page.content),
        `integrations/${slug}.md has no H1 for its index card`
      )
      return {
        href        : `/integrations/${slug}`,
        summaryHtml : renderInlineHtml(md, summary),
        tagline     : requireString(page.data.tagline, fieldMessage(slug, 'tagline')),
        title
      }
    })
  }
})
