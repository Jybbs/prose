import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import * as renderer       from '../markdown/renderer'
import { CATALOG_SUFFIX } from '../shared/constants'

interface KitSource {
  file : string
  html : string
}

declare const data: Readonly<Record<string, KitSource>>
export { data }

export default defineLoader({
  watch: ['../../kit/**/*.{ts,tsx}'],
  async load(files): Promise<Record<string, KitSource>> {
    const md = await renderer.getRenderer()

    return Object.fromEntries(await Promise.all(files.filter(file => file.endsWith(CATALOG_SUFFIX)).map(async sidecar => {
      const stem  = sidecar.slice(0, -CATALOG_SUFFIX.length)
      const piece = files.find(file => file === `${stem}.tsx` || file === `${stem}.ts`)!
      const html  = await renderer.renderFencedHtml(md, fs.readFileSync(piece, 'utf8').trimEnd(), 'tsx')
      return [path.basename(stem), { file: path.basename(piece), html }] as const
    })))
  }
})
