import { SITE_HOSTNAME } from './constants'

export function canonicalUrl(relativePath: string): string {
  const slug = relativePath
    .replace(/(^|\/)index\.md$/, '$1')
    .replace(/\.md$/, '')
  return `${SITE_HOSTNAME}/${slug}`
}
