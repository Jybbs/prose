import fs   from 'node:fs'
import path from 'node:path'

import { hash } from 'ohash'

import { siteDir }             from '../../shared/paths'
import { readPackageVersions } from '../../shared/version'
import type { BrandAssets }    from './assets'
import type { OgPage }         from '../pages'

// Every bare specifier the card modules import sits in this list, and each
// pinned version joins the cache key, so a renderer bump re-renders the cards.
export const CARD_PACKAGES: readonly string[] = [
  '@iconify/utils', '@resvg/resvg-js', 'cacache', 'markdown-it', 'ohash', 'satori'
]

const CARD_VERSIONS = readPackageVersions(siteDir(import.meta.url), CARD_PACKAGES)

const OG_DIR = import.meta.dirname

// Each path the card modules import from outside their own directory, hashed
// alongside the directory itself so an edit to one moves every card key.
export const SHARED_SOURCES: readonly string[] = [
  '../../config/og-url.ts',    '../../shared/constants.ts',  '../../shared/numerals.ts',
  '../../shared/palette.ts',   '../../shared/paths.ts',      '../../shared/registries.ts',
  '../../shared/svg.ts',       '../../shared/version.ts',    '../pages.ts'
]

const TEMPLATE_DIGEST = hash(
  [...fs.readdirSync(OG_DIR, { withFileTypes: true })
    .filter(entry => entry.isFile() && !entry.name.startsWith('.'))
    .map(entry => entry.name).sort(), ...SHARED_SOURCES]
    .map(file => fs.readFileSync(path.join(OG_DIR, file), 'utf8'))
)

type CardInput = OgPage | 'landing'

export function cardKeyer(
  brand    : BrandAssets,
  version  : string,
  packages : Record<string, string> = CARD_VERSIONS
): (card: CardInput) => string {
  const base = { brand: hash(brand), packages, template: TEMPLATE_DIGEST, version }
  return card => hash({ base, card })
}
