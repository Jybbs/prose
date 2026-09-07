import fs   from 'node:fs'
import path from 'node:path'

import { hash } from 'ohash'

import { siteDir }             from '../../shared/paths'
import { readPackageVersions } from '../../shared/version'
import type { BrandAssets }    from './assets'
import type { OgPage }         from '../pages'

// Every bare specifier the card modules import sits in exactly one of these
// two lists. A keyed package reaches the rendered bytes and joins the cache
// key, whereas an unkeyed one hashes or stores the result instead.
export const KEYED_PACKAGES: readonly string[] = [
  '@iconify/utils', '@resvg/resvg-js', 'markdown-it', 'satori'
]

export const UNKEYED_PACKAGES: readonly string[] = ['cacache', 'ohash']

const KEYED_VERSIONS = readPackageVersions(siteDir(import.meta.url), KEYED_PACKAGES)

const OG_DIR = import.meta.dirname

// Each path the card modules import from outside their own directory, hashed
// alongside the directory itself so an edit to one moves every card key.
export const SHARED_SOURCES: readonly string[] = [
  '../../config/og-url.ts',    '../../shared/constants.ts',  '../../shared/numerals.ts',
  '../../shared/palette.ts',   '../../shared/paths.ts',      '../../shared/registries.ts',
  '../../shared/svg.ts',       '../../shared/version.ts',    '../pages.ts'
]

const TEMPLATE_DIGEST = hash(
  [...fs.readdirSync(OG_DIR).filter(file => !file.startsWith('.')).sort(), ...SHARED_SOURCES]
    .map(file => fs.readFileSync(path.join(OG_DIR, file), 'utf8'))
)

type CardInput = OgPage | 'landing'

export function cardKeyer(
  brand    : BrandAssets,
  version  : string,
  packages : Record<string, string> = KEYED_VERSIONS
): (card: CardInput) => string {
  const base = { brand: hash(brand), packages, template: TEMPLATE_DIGEST, version }
  return card => hash({ base, card })
}
