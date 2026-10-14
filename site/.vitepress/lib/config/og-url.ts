import path from 'node:path'

import { SITE_HOSTNAME } from '../shared/constants'
import { sectionOf }     from '../shared/registries'
import { stripSuffix }   from '../shared/strip-suffix'

export const TOUCH_ICON_PATH = 'apple-touch-icon.png'

// Returns the landing card for every page outside a section, the landing page
// included, because the card build draws a page's own card only inside a section.
export function ogImagePath(relativePath: string): string {
  if (sectionOf(relativePath) === null) return 'og.png'
  return path.posix.join('og', `${stripSuffix(relativePath, '.md')}.png`)
}

export function ogImageUrl(relativePath: string): string {
  return `${SITE_HOSTNAME}/${ogImagePath(relativePath)}`
}
