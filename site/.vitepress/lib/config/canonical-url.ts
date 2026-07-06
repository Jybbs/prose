import { SITE_HOSTNAME } from '../shared/constants'

export function canonicalUrl(relativePath: string): string {
  return `${SITE_HOSTNAME}/${pagePath(relativePath)}`
}

export function pagePath(relativePath: string): string {
  return relativePath
    .replace(/(^|\/)index\.md$/, '$1')
    .replace(/\.md$/, '')
}
