import fs   from 'node:fs'
import path from 'node:path'

import { hash } from 'ohash'

import { siteDir }             from '../../shared/paths'
import { readPackageVersions } from '../../shared/version'
import type { BrandAssets }    from './assets'
import type { OgPage }         from '../pages'

const OG_DIR = import.meta.dirname

export const RENDERERS: readonly string[] = ['@resvg/resvg-js', 'satori']

const RENDERER_VERSIONS = readPackageVersions(siteDir(import.meta.url), RENDERERS)

const SHARED_SOURCES: readonly string[] = ['../../shared/palette.ts', '../../shared/registries.ts']

const TEMPLATE_DIGEST = hash(
  [...fs.readdirSync(OG_DIR).filter(file => !file.startsWith('.')).sort(), ...SHARED_SOURCES]
    .map(file => fs.readFileSync(path.join(OG_DIR, file), 'utf8'))
)

type CardInput = OgPage | 'landing'

export function cardKeyer(
  brand     : BrandAssets,
  version   : string,
  renderers : Record<string, string> = RENDERER_VERSIONS
): (card: CardInput) => string {
  const base = { brand: hash(brand), renderers, template: TEMPLATE_DIGEST, version }
  return card => hash({ base, card })
}
