import { readdirSync } from 'node:fs'

// An entry page is a `.md` or `.mdx` file other than a section's `index`, and its
// slug is the basename without that extension.
const PAGE = /\.mdx?$/

const isPage = (file: string): boolean => PAGE.test(file) && !isIndex(file)
export const isIndex = (file: string): boolean => slugOf(file) === 'index'
export const slugOf = (file: string): string => file.replace(PAGE, '')

// Each entry page directly under `dir`, paired with its slug.
export function* pageFiles(dir: string): Iterable<{ file: string, slug: string }> {
  for (const file of readdirSync(dir)) {
    if (isPage(file)) yield { file, slug: slugOf(file) }
  }
}

// The immediate subdirectory names of `dir`, sorted.
export const subdirectories = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).filter(e => e.isDirectory()).map(e => e.name).sort()
