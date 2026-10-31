import fs   from 'node:fs'
import path from 'node:path'

import { isCSSRequest } from 'vite'

import type { Plugin } from 'vite'

const LAYER         = 'components'
const SPECIAL_QUERY = /[?&](?:inline|raw|url)\b/

const layered = (css: string): string => `@layer ${LAYER} {\n${css}\n}`

// Wraps the icon stylesheet VitePress writes into the output's assets after the
// bundle, outside Vite's CSS transform, in the same layer as every other sheet.
export function layerIconStylesheet(assetsDir: string): void {
  for (const file of fs.globSync('vp-icons.*.css', { cwd: assetsDir })) {
    const at = path.join(assetsDir, file)
    fs.writeFileSync(at, layered(fs.readFileSync(at, 'utf8')))
  }
}

// Wraps every stylesheet but the Tailwind entry, the VitePress default theme's
// included, in the layer that entry declares beneath its utilities, so a
// utility outranks each rule while the rules keep their order and specificity
// among themselves.
export function stylesheetLayerPlugin(entry: string): Plugin {
  return {
    name: 'prose-stylesheet-layer',
    transform(code, id) {
      if (isCSSRequest(id) && !SPECIAL_QUERY.test(id) && id.split('?')[0] !== entry) {
        return { code: layered(code), map: null }
      }
    }
  }
}
