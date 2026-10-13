import fs   from 'node:fs'
import path from 'node:path'

import { ogImagePath, TOUCH_ICON_PATH } from '../../config/og-url'
import { cacheDirFrom, crateDirFrom }   from '../../shared/paths'
import { readCargoVersion }             from '../../shared/version'
import { loadBrandAssets }              from './assets'
import * as cache                       from './cache'
import { cardKeyer, readRenderGraph }   from './card-key'
import { enumeratePages }               from '../pages'
import * as pool                        from './pool'

export async function buildOgCards(
  srcDir : string,
  pages  : readonly string[],
  outDir : string
): Promise<void> {
  const repo     = path.dirname(srcDir)
  const brand    = loadBrandAssets(srcDir)
  const version  = readCargoVersion(crateDirFrom(repo))
  const cacheDir = cacheDirFrom(repo, 'og')
  const keyOf    = cardKeyer(brand, await readRenderGraph(), version)
  const taskFor  = (card: pool.Card, outputPath: string): pool.RenderTask =>
    ({ card, key: keyOf(card), outputPath })

  const tasks = [
    taskFor('icon', TOUCH_ICON_PATH),
    taskFor('landing', ogImagePath('index.md')),
    ...enumeratePages(srcDir, pages).map(page => taskFor(page, page.outputPath))
  ]

  const misses: pool.RenderTask[] = []
  await Promise.all(tasks.map(async task => {
    const png = await cache.readCard(cacheDir, task.key)
    if (png) writeDist(outDir, task.outputPath, png)
    else misses.push(task)
  }))

  if (misses.length > 0) {
    const rendered = await pool.renderCards(brand, version, misses)
    await Promise.all(rendered.map(async card => {
      writeDist(outDir, card.outputPath, card.png)
      await cache.writeCard(cacheDir, card.key, Buffer.from(card.png))
    }))
  }

  await cache.pruneCards(cacheDir, tasks.map(task => task.key))
}

function writeDist(outDir: string, outputPath: string, png: Uint8Array): void {
  const dest = path.join(outDir, outputPath)
  fs.mkdirSync(path.dirname(dest), { recursive: true })
  fs.writeFileSync(dest, png)
}
