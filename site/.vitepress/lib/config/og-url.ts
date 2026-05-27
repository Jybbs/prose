import path from 'node:path'

import { SITE_HOSTNAME } from '../shared/constants'

export function ogImagePath(relativePath: string): string {
  if (relativePath === 'index.md') return 'og.png'
  return path.posix.join('og', relativePath.replace(/\.md$/, '.png'))
}

export function ogImageUrl(relativePath: string): string {
  return `${SITE_HOSTNAME}/${ogImagePath(relativePath)}`
}
