import path from 'node:path'

import { SITE_HOSTNAME } from '../shared/constants'
import { stripSuffix }   from '../shared/strip-suffix'

export function ogImagePath(relativePath: string): string {
  if (relativePath === 'index.md') return 'og.png'
  return path.posix.join('og', `${stripSuffix(relativePath, '.md')}.png`)
}

export function ogImageUrl(relativePath: string): string {
  return `${SITE_HOSTNAME}/${ogImagePath(relativePath)}`
}
