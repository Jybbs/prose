import path from 'node:path'

import { SITE_HOSTNAME } from '../shared/constants'
import { sectionOf }     from '../shared/registries'
import { stripSuffix }   from '../shared/strip-suffix'

// Returns the landing card for the landing page and for every other page
// outside a section, because the card build draws a card only for a section's page.
export function ogImagePath(relativePath: string): string {
  if (sectionOf(relativePath) === null) return 'og.png'
  return path.posix.join('og', `${stripSuffix(relativePath, '.md')}.png`)
}

export function ogImageUrl(relativePath: string): string {
  return `${SITE_HOSTNAME}/${ogImagePath(relativePath)}`
}
