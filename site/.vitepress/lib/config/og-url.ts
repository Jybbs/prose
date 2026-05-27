import { SITE_HOSTNAME } from '../shared/constants'

export function ogImageUrl(relativePath: string): string {
  if (relativePath === 'index.md') return `${SITE_HOSTNAME}/og.png`
  return `${SITE_HOSTNAME}/og/${relativePath.replace(/\.md$/, '.png')}`
}
