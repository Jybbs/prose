import fs   from 'node:fs'
import path from 'node:path'

import matter     from 'gray-matter'
import MarkdownIt from 'markdown-it'

import { siteDir }    from '../../lib/shared/paths'
import { TOOL_SEEDS } from '../../lib/shared/tools'

const CHIP = /<Tool slug="([a-z]+)"/g

const SITE = siteDir(import.meta.url)

const md = new MarkdownIt({ html: true })

const PAGES = fs.globSync('**/*.md', { cwd: SITE, exclude: name => /^(\.vitepress|node_modules|prototypes)$/.test(name) }).sort()

const TOOLS = Object.entries(TOOL_SEEDS)
  .filter(([, seed]) => !('language' in seed))
  .map(([slug, seed]) => ({
    href    : seed.href,
    pattern : new RegExp(`\\b${RegExp.escape(seed.name)}(?![-\\w])`),
    slug
  }))

// Lists each tool whose first mention in a page's prose is neither its chip nor
// a link to one of the site's own pages, a heading and a code span counting as
// no mention and a link to the tool's own site counting as an unchipped one.
function unchipped(page: string): string[] {
  const seen  = new Set<string>()
  const found = [] as string[]
  const mark  = (slug: string) => seen.add(slug)

  let heading = false
  for (const block of md.parse(matter.read(path.join(SITE, page)).content, {})) {
    if (block.type === 'heading_open') heading = true
    if (block.type === 'heading_close') heading = false
    if (block.type === 'html_block') for (const [, slug] of block.content.matchAll(CHIP)) mark(slug!)
    if (block.type !== 'inline' || heading) continue

    let href = ''
    for (const token of block.children ?? []) {
      if (token.type === 'link_open') href = String(token.attrGet('href') ?? '')
      if (token.type === 'link_close') href = ''
      if (token.type === 'html_inline') for (const [, slug] of token.content.matchAll(CHIP)) mark(slug!)
      if (token.type !== 'text' && !(token.type === 'code_inline' && href)) continue
      for (const tool of TOOLS) {
        if (seen.has(tool.slug) || !tool.pattern.test(token.content)) continue
        mark(tool.slug)
        if (!href || tool.href.startsWith(new URL(href, 'https://site.invalid').origin)) {
          found.push(`${tool.slug}, line ${(block.map?.[0] ?? 0) + 1}: ${token.content.trim().slice(0, 60)}`)
        }
      }
    }
  }
  return found
}

describe('the tool chips', () => {
  it.each(PAGES)('%s marks the first mention of each tool with its chip', page => {
    expect(unchipped(page)).toStrictEqual([])
  })
})
