import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

interface ContentPageMatter {
  content : string
  data    : Record<string, unknown>
  file    : string
  slug    : string
}

// Sorted explicitly because readdir order is platform-dependent.
export function contentPages(directory: string): string[] {
  return fs.readdirSync(directory).filter(isContentPage).sort()
}

export const isContentPage = (name: string): boolean =>
  name.endsWith('.md') && name !== 'index.md'

export function matterPages(directory: string): ContentPageMatter[] {
  return contentPages(directory).map(file => {
    const { content, data } = matter.read(path.join(directory, file))
    return { content, data, file, slug: path.basename(file, '.md') }
  })
}
