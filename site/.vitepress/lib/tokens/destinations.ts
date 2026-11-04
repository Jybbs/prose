import fs   from 'node:fs'
import path from 'node:path'

import { slugify } from '@mdit-vue/shared'

import { markdownH1 } from '../markdown/h1'

// Names an anchored target in words, `code` marking one set as code.
export interface Anchored {
  code : boolean
  text : string
}

// Names where a link leads, the page's title beside the heading or rule its
// anchor points at, or no part where the link names the page alone.
export interface Destination {
  code  : boolean
  part  : string | null
  title : string
}

const HEADING = /^#{2,6} (.+?)\s*$/

// Reads each heading of a page's Markdown outside its code fences, keyed by
// the id VitePress gives it, as plain words.
export function headingsOf(content: string): Map<string, Anchored> {
  const headings = new Map<string, Anchored>()
  let fenced     = false
  for (const line of content.split('\n')) {
    if (line.startsWith('```')) fenced = !fenced
    const heading = fenced ? null : HEADING.exec(line)?.[1]
    if (!heading) continue
    const text = heading.replaceAll(/\[([^\]]*)\]\([^)]*\)/g, '$1').replaceAll(/[`*]/g, '')
    headings.set(slugify(text), { code: /^`[^`]+`$/.test(heading), text })
  }
  return headings
}

// Resolves where each link a page tree holds leads, reading each page's title
// from its H1 and each anchor from the page's headings or, for an anchor no
// heading carries, from `anchors`, keyed by the whole link. A link whose page
// or anchor resolves to nothing throws, so the build fails on a stale link.
export function destinations(siteRoot: string, anchors: ReadonlyMap<string, Anchored>): (href: string) => Destination {
  const pages = new Map<string, { headings: Map<string, Anchored>, title: string }>()
  const page  = (route: string) => {
    const file    = path.join(siteRoot, route.endsWith('/') ? `${route}index.md` : `${route}.md`)
    const content = fs.readFileSync(file, 'utf8')
    const title   = markdownH1(content)
    if (!title) throw new Error(`${route} has no H1 to name it by`)
    return { headings: headingsOf(content), title }
  }
  return href => {
    const [route, anchor] = href.split('#') as [string, string | undefined]
    const read = pages.get(route) ?? page(route)
    pages.set(route, read)
    if (anchor === undefined) return { code: false, part: null, title: read.title }
    const part = read.headings.get(anchor) ?? anchors.get(href)
    if (!part) throw new Error(`${href} names no heading or anchor on its page`)
    return { code: part.code, part: part.text, title: read.title }
  }
}
