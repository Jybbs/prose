import fs from 'node:fs'

// Sorted explicitly because readdir order is platform-dependent.
export function contentPages(directory: string): string[] {
  return fs.readdirSync(directory).filter(isContentPage).sort()
}

export const isContentPage = (name: string): boolean =>
  name.endsWith('.md') && name !== 'index.md'
