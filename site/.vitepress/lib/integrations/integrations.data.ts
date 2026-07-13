import path from 'node:path'

import { defineLoader } from 'vitepress'

import { markdownH1 }                    from '../markdown/h1'
import { inlineNodes, type InlineNode }  from '../markdown/inline-nodes'
import { getRenderer }                   from '../markdown/renderer'
import { matterPages }                   from '../shared/content-page'
import { siteDir }                       from '../shared/paths'
import { requireString }                 from '../shared/require-string'

interface IntegrationCard {
  href         : string
  summaryNodes : InlineNode[]
  tagline      : string
  title        : string
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
    return matterPages(directory).map(({ content, data: fm, slug }) => {
      const summary = requireString(fm.summary, fieldMessage(slug, 'summary'))
      const title   = requireString(
        markdownH1(content),
        `integrations/${slug}.md has no H1 for its index card`
      )
      return {
        href         : `/integrations/${slug}`,
        summaryNodes : inlineNodes(md, summary),
        tagline      : requireString(fm.tagline, fieldMessage(slug, 'tagline')),
        title        : title
      }
    })
  }
})
